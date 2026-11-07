//! Leitura incremental dos transcripts: cada `.jsonl` é lido inteiro uma vez e, nas consultas
//! seguintes, só os bytes que o CLI acrescentou desde então.
//!
//! Os transcripts só crescem (o CLI sempre escreve no fim), então basta guardar até onde cada
//! arquivo já foi lido e o que se extraiu dele. Sem isso, cada consulta reabria os arquivos das
//! últimas 48 h e relia tudo, e as linhas de um transcript podem ter megabytes. O estado de um
//! arquivo que saiu da janela é descartado, o que mantém a memória limitada.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Uma linha maior que isso é descartada em vez de acumulada: nenhum registro de uso chega perto.
const MAX_LINE: usize = 32 * 1024 * 1024;

const CHUNK: usize = 64 * 1024;

struct Entry<S> {
    /// Quantos bytes do arquivo já foram consumidos.
    offset: u64,
    /// O começo de uma linha cujo fim o CLI ainda não escreveu.
    pending: Vec<u8>,
    /// A linha em curso passou de `MAX_LINE`: o resto dela é ignorado até o próximo `\n`.
    oversized: bool,
    state: S,
}

impl<S: Default> Entry<S> {
    fn new() -> Self {
        Self { offset: 0, pending: Vec::new(), oversized: false, state: S::default() }
    }
}

impl<S> Entry<S> {
    fn feed(&mut self, bytes: &[u8], on_line: &mut impl FnMut(&mut S, &str)) {
        for part in bytes.split_inclusive(|b| *b == b'\n') {
            let complete = part.ends_with(b"\n");
            // O caso comum: a linha inteira está dentro do bloco, então não precisa de cópia.
            if complete && self.pending.is_empty() && !self.oversized {
                if let Ok(line) = std::str::from_utf8(part) {
                    on_line(&mut self.state, line);
                }
                continue;
            }
            if !self.oversized && self.pending.len() + part.len() <= MAX_LINE {
                self.pending.extend_from_slice(part);
            } else {
                self.pending.clear();
                self.oversized = true;
            }
            if complete {
                if !self.oversized {
                    if let Ok(line) = std::str::from_utf8(&self.pending) {
                        on_line(&mut self.state, line);
                    }
                }
                self.pending.clear();
                self.oversized = false;
                if self.pending.capacity() > 1 << 20 {
                    self.pending = Vec::new();
                }
            }
        }
    }
}

/// O que se extraiu de cada arquivo (`S`) e até onde ele foi lido.
pub(crate) struct FileCache<S> {
    files: HashMap<PathBuf, Entry<S>>,
}

impl<S> Default for FileCache<S> {
    fn default() -> Self {
        Self { files: HashMap::new() }
    }
}

impl<S: Default> FileCache<S> {
    /// Põe em dia o estado dos arquivos de `paths`: lê só o que cresceu desde a última vez, linha a
    /// linha (`feed`), e esquece os que não estão mais na lista. Um arquivo que encolheu foi
    /// reescrito, e então é relido do começo. Uma linha sem `\n` no fim espera o resto ser escrito.
    pub fn refresh(&mut self, paths: Vec<PathBuf>, mut feed: impl FnMut(&mut S, &str)) {
        let keep: HashSet<&PathBuf> = paths.iter().collect();
        self.files.retain(|path, _| keep.contains(path));

        let mut chunk = vec![0u8; CHUNK];
        for path in &paths {
            let Ok(mut file) = std::fs::File::open(path) else { continue };
            let Ok(len) = file.metadata().map(|m| m.len()) else { continue };
            let entry = self.files.entry(path.clone()).or_insert_with(Entry::new);
            if len < entry.offset {
                *entry = Entry::new();
            }
            if len == entry.offset || file.seek(SeekFrom::Start(entry.offset)).is_err() {
                continue;
            }
            while let Ok(n) = file.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                entry.offset += n as u64;
                entry.feed(&chunk[..n], &mut feed);
            }
        }
    }

    /// O estado de cada arquivo, em ordem de caminho: o resultado não depende da ordem em que o
    /// sistema de arquivos listou.
    pub fn states(&self) -> Vec<&S> {
        let mut entries: Vec<_> = self.files.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries.into_iter().map(|(_, entry)| &entry.state).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pier-ai-files-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn collect(state: &mut Vec<String>, line: &str) {
        state.push(line.trim_end().to_string());
    }

    fn lines(cache: &FileCache<Vec<String>>) -> Vec<String> {
        cache.states().into_iter().flatten().cloned().collect()
    }

    fn append(path: &PathBuf, bytes: &[u8]) {
        std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap().write_all(bytes).unwrap();
    }

    #[test]
    fn reads_only_what_was_appended() {
        let dir = dir("append");
        let path = dir.join("a.jsonl");
        append(&path, b"one\ntwo\n");
        let mut cache = FileCache::default();
        cache.refresh(vec![path.clone()], collect);
        assert_eq!(lines(&cache), ["one", "two"]);

        // Sem mudança nada é entregue de novo, e o que cresce chega uma vez só.
        cache.refresh(vec![path.clone()], collect);
        append(&path, b"three\n");
        cache.refresh(vec![path.clone()], collect);
        assert_eq!(lines(&cache), ["one", "two", "three"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unfinished_line_waits_for_its_end() {
        let dir = dir("partial");
        let path = dir.join("a.jsonl");
        append(&path, b"one\nhal");
        let mut cache = FileCache::default();
        cache.refresh(vec![path.clone()], collect);
        assert_eq!(lines(&cache), ["one"]);
        append(&path, b"f\ntwo\n");
        cache.refresh(vec![path.clone()], collect);
        assert_eq!(lines(&cache), ["one", "half", "two"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_that_shrank_is_read_again_from_the_start() {
        let dir = dir("shrink");
        let path = dir.join("a.jsonl");
        append(&path, b"one\ntwo\n");
        let mut cache = FileCache::default();
        cache.refresh(vec![path.clone()], collect);
        std::fs::write(&path, b"x\n").unwrap();
        cache.refresh(vec![path.clone()], collect);
        assert_eq!(lines(&cache), ["x"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn forgets_files_that_left_the_list() {
        let dir = dir("forget");
        let (a, b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
        append(&a, b"a\n");
        append(&b, b"b\n");
        let mut cache = FileCache::default();
        cache.refresh(vec![a.clone(), b.clone()], collect);
        assert_eq!(lines(&cache), ["a", "b"]);
        cache.refresh(vec![b.clone()], collect);
        assert_eq!(lines(&cache), ["b"]);
        // Se ele volta, é lido do zero.
        cache.refresh(vec![a, b], collect);
        assert_eq!(lines(&cache), ["a", "b"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn skips_a_line_that_is_not_utf8_and_keeps_going() {
        let dir = dir("utf8");
        let path = dir.join("a.jsonl");
        append(&path, b"ok\n\xff\xfe\nnext\n");
        let mut cache = FileCache::default();
        cache.refresh(vec![path], collect);
        assert_eq!(lines(&cache), ["ok", "next"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_line_longer_than_a_chunk_arrives_whole() {
        let dir = dir("long");
        let path = dir.join("a.jsonl");
        let long = "x".repeat(3 * CHUNK + 17);
        append(&path, format!("{long}\nshort\n").as_bytes());
        let mut cache = FileCache::default();
        cache.refresh(vec![path], collect);
        assert_eq!(lines(&cache), [long.as_str(), "short"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn states_come_in_path_order() {
        let dir = dir("order");
        let (a, b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
        append(&a, b"a\n");
        append(&b, b"b\n");
        let mut cache = FileCache::default();
        cache.refresh(vec![b, a], collect);
        assert_eq!(lines(&cache), ["a", "b"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
