// wasm-bindgen output lives in public/ and is loaded at runtime by URL, so it has no
// static specifier to declare. App.tsx types the loaded module structurally instead.
export type GameWasmModule = {
  default(): Promise<unknown>;
  game_state_json(points: number): string;
};
