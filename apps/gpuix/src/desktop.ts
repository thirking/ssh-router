export interface TitlebarRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Desktop {
  acquireInstance(name?: string): boolean;
  attachWindow(): boolean;
  setTitlebarRegion(region: TitlebarRegion | null): void;
  isWindowMaximized(): boolean;
  toggleMaximizeWindow(): void;
  closeWindow(): void;
}
