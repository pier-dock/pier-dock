import { Briefcase, Code, Gamepad2, House, Layers, Moon, Music, Target, type LucideIcon } from "lucide-react";

export const PROFILE_ICONS: Record<string, LucideIcon> = {
  layers: Layers,
  briefcase: Briefcase,
  house: House,
  target: Target,
  code: Code,
  music: Music,
  moon: Moon,
  gamepad: Gamepad2,
};

export const profileIcon = (name: string) => PROFILE_ICONS[name] ?? Layers;
