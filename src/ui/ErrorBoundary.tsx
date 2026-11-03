import { Component, type ReactNode } from "react";
import { useT } from "../i18n";

function RenderError({ retry }: { retry: () => void }) {
  const t = useT().common;
  return (
    <div className="load-error" role="alert">
      <span>{t.sectionFailed}</span>
      <button className="load-error-retry" onClick={retry}>
        {t.tryAgain}
      </button>
    </div>
  );
}

/** Um erro ao renderizar fica contido aqui; sem isso, o React desmonta a janela inteira. */
export class ErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (this.state.failed) return <RenderError retry={() => this.setState({ failed: false })} />;
    return this.props.children;
  }
}
