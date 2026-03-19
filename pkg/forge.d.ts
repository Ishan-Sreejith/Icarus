


export class CoReEngine {
    free(): void;
    [Symbol.dispose](): void;
    execute(source: string): string;
    get_features(): string;
    get_version(): string;
    constructor();
    reset(): void;
}

export function format_error(error: string): string;

export function get_sample_code(): string;

export function main(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly rt_release: (a: bigint) => void;
    readonly rt_index_set: (a: bigint, b: bigint, c: bigint) => void;
    readonly rt_map_set: (a: bigint, b: bigint, c: bigint) => void;
    readonly rt_list_len: (a: bigint) => bigint;
    readonly rt_to_str: (a: bigint) => bigint;
    readonly rt_to_num: (a: bigint) => bigint;
    readonly rt_map_keys: (a: bigint) => bigint;
    readonly rt_map_values: (a: bigint) => bigint;
    readonly rt_list_pop: (a: bigint) => bigint;
    readonly rt_abs: (a: bigint) => bigint;
    readonly rt_sqrt: (a: bigint) => bigint;
    readonly rt_is_truthy: (a: bigint) => bigint;
    readonly rt_min: (a: bigint, b: bigint) => bigint;
    readonly rt_max: (a: bigint, b: bigint) => bigint;
    readonly rt_pow: (a: bigint, b: bigint) => bigint;
    readonly rt_contains: (a: bigint, b: bigint) => bigint;
    readonly rt_list_push: (a: bigint, b: bigint) => void;
    readonly rt_range: (a: bigint, b: bigint) => bigint;
    readonly rt_print: (a: bigint) => void;
    readonly rt_map_get: (a: bigint, b: bigint) => bigint;
    readonly rt_alloc_map: () => bigint;
    readonly rt_index_get: (a: bigint, b: bigint) => bigint;
    readonly rt_alloc_list: (a: bigint) => bigint;
    readonly rt_retain: (a: bigint) => void;
    readonly rt_alloc_string: (a: number, b: bigint) => bigint;
    readonly rt_float_div: (a: bigint, b: bigint) => bigint;
    readonly rt_float_mul: (a: bigint, b: bigint) => bigint;
    readonly rt_float_sub: (a: bigint, b: bigint) => bigint;
    readonly rt_float_add: (a: bigint, b: bigint) => bigint;
    readonly rt_not: (a: bigint) => bigint;
    readonly rt_or: (a: bigint, b: bigint) => bigint;
    readonly rt_and: (a: bigint, b: bigint) => bigint;
    readonly rt_gt: (a: bigint, b: bigint) => bigint;
    readonly rt_lt: (a: bigint, b: bigint) => bigint;
    readonly rt_ne: (a: bigint, b: bigint) => bigint;
    readonly rt_eq: (a: bigint, b: bigint) => bigint;
    readonly rt_div: (a: bigint, b: bigint) => bigint;
    readonly rt_mul: (a: bigint, b: bigint) => bigint;
    readonly rt_sub: (a: bigint, b: bigint) => bigint;
    readonly rt_add: (a: bigint, b: bigint) => bigint;
    readonly __wbg_coreengine_free: (a: number, b: number) => void;
    readonly coreengine_execute: (a: number, b: number, c: number) => [number, number];
    readonly coreengine_get_features: (a: number) => [number, number];
    readonly coreengine_get_version: (a: number) => [number, number];
    readonly coreengine_new: () => number;
    readonly coreengine_reset: (a: number) => void;
    readonly format_error: (a: number, b: number) => [number, number];
    readonly get_sample_code: () => [number, number];
    readonly main: () => void;
    readonly rt_alloc_float: (a: number) => bigint;
    readonly rt_file_close: (a: bigint) => void;
    readonly rt_file_open: (a: bigint) => bigint;
    readonly rt_file_read: (a: bigint) => bigint;
    readonly rt_get_last_error: () => bigint;
    readonly rt_list_set: (a: bigint, b: bigint, c: bigint) => void;
    readonly rt_list_get: (a: bigint, b: bigint) => bigint;
    readonly rt_pop_try: () => void;
    readonly rt_push_try: (a: bigint, b: bigint, c: bigint) => void;
    readonly rt_throw: (a: bigint) => bigint;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;


export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;


export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
