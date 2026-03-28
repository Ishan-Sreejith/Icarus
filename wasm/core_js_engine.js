export class CoReJsEngine {
    constructor() {
        this.version = "Icarus Language v1.0 (JavaScript Engine)";
    }

    get_version() {
        return this.version;
    }

    get_features() {
        return JSON.stringify([
            "JavaScript Engine",
            "Direct V8 Execution",
            "No WebAssembly Required",
            "Icarus Syntax Transpilation",
            "Builtin Plugins",
            "Virtual File System"
        ]);
    }

    get_sample_code() {
        return `var greeting: "Hello from the Icarus JS Engine!"
say: greeting

var numbers: [1, 2, 3, 4, 5]
var i: 0
var sum: 0
while i < len(numbers) {
    sum: sum + numbers[i]
    i: i + 1
}
say: "Sum: " + str(sum)`;
    }

    execute(source) {
        const out = [];
        const js = this.transpile(source);
        let fn;
        try {
            fn = new Function(
                "__out",
                "__len",
                "__str",
                "__num",
                "__upper",
                "__lower",
                "__range",
                "__push",
                "__pop",
                "__contains",
                "__is_map",
                "__is_list",
                "__is_string",
                "__bool",
                "__type",
                "__keys",
                "__values",
                js
            );
        } catch (err) {
            throw new Error(`Transpile/parse error: ${err.message}\nGenerated JS:\n${js}`);
        }
        fn(
            out,
            (v) => {
                if (Array.isArray(v) || typeof v === "string") return v.length;
                if (v && typeof v === "object") return Object.keys(v).length;
                return 0;
            },
            (v) => String(v),
            (v) => {
                const n = Number(v);
                return Number.isFinite(n) ? n : 0;
            },
            (v) => String(v).toUpperCase(),
            (v) => String(v).toLowerCase(),
            (start, end) => {
                const s = Number(start);
                const e = Number(end);
                if (!Number.isFinite(s) || !Number.isFinite(e)) return [];
                const out = [];
                for (let i = s; i < e; i++) out.push(i);
                return out;
            },
            (list, item) => {
                if (!Array.isArray(list)) return 0;
                list.push(item);
                return list.length;
            },
            (list) => {
                if (!Array.isArray(list)) return 0;
                return list.pop();
            },
            (hay, needle) => {
                if (typeof hay === "string") return hay.includes(String(needle));
                if (Array.isArray(hay)) return hay.includes(needle);
                if (hay && typeof hay === "object") return Object.prototype.hasOwnProperty.call(hay, String(needle));
                return false;
            },
            (v) => !!(v && typeof v === "object" && !Array.isArray(v)),
            (v) => Array.isArray(v),
            (v) => typeof v === "string",
            (v) => !!v,
            (v) => {
                if (Array.isArray(v)) return "list";
                if (v && typeof v === "object") return "map";
                if (typeof v === "string") return "string";
                if (typeof v === "number") return "number";
                if (typeof v === "boolean") return "bool";
                return "null";
            },
            (v) => {
                if (v && typeof v === "object" && !Array.isArray(v)) return Object.keys(v);
                return [];
            },
            (v) => {
                if (v && typeof v === "object" && !Array.isArray(v)) return Object.values(v);
                return [];
            }
        );
        return out.length ? out.join("\n") : "Program executed successfully";
    }

    transpile(source) {
        const lines = source.replace(/\r\n/g, "\n").split("\n");
        const fnNames = this.collectFunctionNames(lines);
        const builtinCalls = new Set([
            "len",
            "str",
            "num",
            "upper",
            "lower",
            "range",
            "push",
            "pop",
            "contains",
            "is_map",
            "is_list",
            "is_string",
            "bool",
            "type",
            "keys",
            "values"
        ]);
        const prelude = [
            "const len = __len;",
            "const str = __str;",
            "const num = __num;",
            "const upper = __upper;",
            "const lower = __lower;",
            "const range = __range;",
            "const push = __push;",
            "const pop = __pop;",
            "const contains = __contains;",
            "const is_map = __is_map;",
            "const is_list = __is_list;",
            "const is_string = __is_string;",
            "const bool = __bool;",
            "const type = __type;",
            "const keys = __keys;",
            "const values = __values;"
        ];
        const out = [];
        for (let raw of lines) {
            const line = raw.trim();
            if (!line || line.startsWith("//") || line.startsWith("#")) continue;
            const classMatch = line.match(/^(cl|clg|clc)\s+[A-Za-z_]\w*\s*\{.*\}$/);
            if (classMatch) {
                continue;
            }
            if (line.startsWith("say:")) {
                const expr = this.rewriteExpr(line.slice(4).trim());
                out.push(`__out.push(String(${expr}));`);
                continue;
            }
            if (line.startsWith("var ")) {
                const rest = line.slice(4);
                const p = rest.indexOf(":");
                if (p >= 0) {
                    const name = rest.slice(0, p).trim();
                    const expr = this.rewriteExpr(rest.slice(p + 1).trim());
                    if (expr.endsWith("{")) {
                        out.push(`let ${name} = ${expr}`);
                    } else {
                        out.push(`let ${name} = ${expr};`);
                    }
                    continue;
                }
            }
            const fnInlineMatch = line.match(/^(fn|fng|fnc)\s+([A-Za-z_]\w*)\s*:\s*(.*?)\s*\{\s*(.+)\s*\}$/);
            if (fnInlineMatch) {
                const name = fnInlineMatch[2];
                const args = fnInlineMatch[3].trim();
                const body = fnInlineMatch[4].trim();
                if (body.startsWith("return ")) {
                    out.push(`function ${name}(${args}) { return ${this.rewriteExpr(body.slice(7).trim())}; }`);
                } else {
                    out.push(`function ${name}(${args}) { ${this.rewriteExpr(body)}; }`);
                }
                continue;
            }
            const fnMatch = line.match(/^(fn|fng|fnc)\s+([A-Za-z_]\w*)\s*:\s*(.*?)\s*\{$/);
            if (fnMatch) {
                const name = fnMatch[2];
                const args = fnMatch[3].trim();
                out.push(`function ${name}(${args}) {`);
                continue;
            }
            const ifMatch = line.match(/^if\s+(.+)\s*\{$/);
            if (ifMatch) {
                out.push(`if (${this.rewriteExpr(ifMatch[1].trim())}) {`);
                continue;
            }
            const whileMatch = line.match(/^while\s+(.+)\s*\{$/);
            if (whileMatch) {
                out.push(`while (${this.rewriteExpr(whileMatch[1].trim())}) {`);
                continue;
            }
            const forMatch = line.match(/^for\s+([A-Za-z_]\w*)\s+in\s+(.+)\.\.(.+)\s*\{$/);
            if (forMatch) {
                const name = forMatch[1];
                const startExpr = this.rewriteExpr(forMatch[2].trim());
                const endExpr = this.rewriteExpr(forMatch[3].trim());
                out.push(`for (let ${name} = ${startExpr}; ${name} < ${endExpr}; ${name}++) {`);
                continue;
            }
            if (line === "}" || line === "else {" || line === "} else {") {
                out.push(line);
                continue;
            }
            if (line.startsWith("return ")) {
                out.push(`return ${this.rewriteExpr(line.slice(7).trim())};`);
                continue;
            }
            const callMatch = line.match(/^([A-Za-z_]\w*)\s*:\s*(.*)$/);
            if (callMatch) {
                const name = callMatch[1];
                const rhs = callMatch[2].trim();
                if (name !== "if" && name !== "while" && name !== "for") {
                    if (!rhs) {
                        out.push(`${name}();`);
                        continue;
                    }
                    if (fnNames.has(name) || builtinCalls.has(name)) {
                        const args = this.normalizeCallArgs(rhs);
                        out.push(`${name}(${this.rewriteExpr(args)});`);
                    } else {
                        out.push(`${name} = ${this.rewriteExpr(rhs)};`);
                    }
                    continue;
                }
            }
            out.push(this.rewriteStatement(line));
        }
        return prelude.join("\n") + "\n" + out.join("\n");
    }

    rewriteExpr(expr) {
        const converted = this.convertColonCalls(expr);
        const logical = this.replaceLogicalOps(converted);
        return logical
            .replace(/\blen\s*\(/g, "__len(")
            .replace(/\bstr\s*\(/g, "__str(")
            .replace(/\bnum\s*\(/g, "__num(")
            .replace(/\bupper\s*\(/g, "__upper(")
            .replace(/\blower\s*\(/g, "__lower(")
            .replace(/\bkeys\s*\(/g, "__keys(")
            .replace(/\bvalues\s*\(/g, "__values(");
    }

    convertColonCalls(expr) {
        if (!expr) return expr;
        const idx = this.findTopLevelColon(expr);
        if (idx === -1) return expr;
        const name = expr.slice(0, idx).trim();
        if (!/^[A-Za-z_]\w*$/.test(name)) return expr;
        const args = expr.slice(idx + 1).trim();
        if (!args) return `${name}()`;
        const normalized = this.normalizeCallArgs(args);
        return `${name}(${normalized})`;
    }

    rewriteStatement(line) {
        if (line.endsWith("{") || line.endsWith("}") || line.endsWith(";") || line.endsWith(",")) return line;
        if (/^["'][^"']+["']\s*:/.test(line)) return line;
        return `${this.rewriteExpr(line)};`;
    }

    collectFunctionNames(lines) {
        const names = new Set();
        for (const raw of lines) {
            const line = raw.trim();
            const match = line.match(/^(fn|fng|fnc)\s+([A-Za-z_]\w*)\s*:/);
            if (match) names.add(match[2]);
        }
        return names;
    }

    replaceLogicalOps(expr) {
        let out = "";
        let inString = false;
        let quote = "";
        let escape = false;
        for (let i = 0; i < expr.length; i++) {
            const ch = expr[i];
            if (inString) {
                out += ch;
                if (escape) {
                    escape = false;
                } else if (ch === "\\") {
                    escape = true;
                } else if (ch === quote) {
                    inString = false;
                    quote = "";
                }
                continue;
            }
            if (ch === "\"" || ch === "'") {
                inString = true;
                quote = ch;
                out += ch;
                continue;
            }
            if (/[A-Za-z_]/.test(ch)) {
                let j = i + 1;
                while (j < expr.length && /[A-Za-z0-9_]/.test(expr[j])) j++;
                const word = expr.slice(i, j);
                if (word === "and") out += "&&";
                else if (word === "or") out += "||";
                else if (word === "not") out += "!";
                else out += word;
                i = j - 1;
                continue;
            }
            out += ch;
        }
        return out;
    }

    findTopLevelColon(expr) {
        let depth = 0;
        let inString = false;
        let quote = "";
        let escape = false;
        for (let i = 0; i < expr.length; i++) {
            const ch = expr[i];
            if (inString) {
                if (escape) {
                    escape = false;
                } else if (ch === "\\") {
                    escape = true;
                } else if (ch === quote) {
                    inString = false;
                    quote = "";
                }
                continue;
            }
            if (ch === "\"" || ch === "'") {
                inString = true;
                quote = ch;
                continue;
            }
            if (ch === "(" || ch === "[" || ch === "{") depth += 1;
            if (ch === ")" || ch === "]" || ch === "}") depth = Math.max(0, depth - 1);
            if (ch === ":" && depth === 0) return i;
        }
        return -1;
    }

    hasTopLevelComma(expr) {
        let depth = 0;
        let inString = false;
        let quote = "";
        let escape = false;
        for (let i = 0; i < expr.length; i++) {
            const ch = expr[i];
            if (inString) {
                if (escape) {
                    escape = false;
                } else if (ch === "\\") {
                    escape = true;
                } else if (ch === quote) {
                    inString = false;
                    quote = "";
                }
                continue;
            }
            if (ch === "\"" || ch === "'") {
                inString = true;
                quote = ch;
                continue;
            }
            if (ch === "(" || ch === "[" || ch === "{") depth += 1;
            if (ch === ")" || ch === "]" || ch === "}") depth = Math.max(0, depth - 1);
            if (ch === "," && depth === 0) return true;
        }
        return false;
    }

    normalizeCallArgs(args) {
        if (!args) return "";
        if (this.hasTopLevelComma(args)) return args;
        const parts = this.splitArgsByWhitespace(args);
        if (parts.length <= 1) return args;
        return parts.join(", ");
    }

    splitArgsByWhitespace(expr) {
        const parts = [];
        let current = "";
        let depth = 0;
        let inString = false;
        let quote = "";
        let escape = false;
        for (let i = 0; i < expr.length; i++) {
            const ch = expr[i];
            if (inString) {
                current += ch;
                if (escape) {
                    escape = false;
                } else if (ch === "\\") {
                    escape = true;
                } else if (ch === quote) {
                    inString = false;
                    quote = "";
                }
                continue;
            }
            if (ch === "\"" || ch === "'") {
                inString = true;
                quote = ch;
                current += ch;
                continue;
            }
            if (ch === "(" || ch === "[" || ch === "{") {
                depth += 1;
                current += ch;
                continue;
            }
            if (ch === ")" || ch === "]" || ch === "}") {
                depth = Math.max(0, depth - 1);
                current += ch;
                continue;
            }
            if (depth === 0 && /\s/.test(ch)) {
                if (current) {
                    parts.push(current);
                    current = "";
                }
                continue;
            }
            current += ch;
        }
        if (current) parts.push(current);
        return parts;
    }
}
