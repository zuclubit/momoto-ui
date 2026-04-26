import * as wasm from "./momoto_wasm_bg.wasm";
export * from "./momoto_wasm_bg.js";
import { __wbg_set_wasm } from "./momoto_wasm_bg.js";
__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
