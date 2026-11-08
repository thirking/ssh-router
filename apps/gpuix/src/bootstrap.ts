import { existsSync } from "node:fs";
import { dirname, join } from "node:path";

const native = join(
  dirname(process.execPath),
  "resources",
  "gpuix-native.node",
);
if (existsSync(native)) process.env.NAPI_RS_NATIVE_LIBRARY_PATH = native;
await import("./main");
