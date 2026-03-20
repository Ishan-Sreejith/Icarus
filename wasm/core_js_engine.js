export class CoReJsEngine {
    constructor() {
        this.version = "CoRe Language v1.0 (JavaScript Engine)";
    }

    get_version() {
        return this.version;
    }

    get_features() {
        return JSON.stringify([
            "JavaScript Engine",
            "Direct V8 Execution",
            "No WebAssembly Required",
            "CoRe Syntax Transpilation",
            "Builtin Plugins",
            "Virtual File System"
        ]);
    }

    get_sample_code() {
        return `var greeting: "Hello from CoRe JS Engine!"
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
            }
        );
        return out.length ? out.join("\n") : "Program executed successfully";
    }

    transpile(source) {
        const lines = source.replace(/\r\n/g, "\n").split("\n");
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
            "const type = __type;"
        ];
        const out = [];
        for (let raw of lines) {
            const line = raw.trim();
            if (!line || line.startsWith("//") || line.startsWith("#")) continue;
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
            const fnMatch = line.match(/^fn\s+([A-Za-z_]\w*)\s*:\s*(.*?)\s*\{$/);
            if (fnMatch) {
                const name = fnMatch[1];
                const args = fnMatch[2].trim();
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
                    if (/^[A-Za-z_]\w*$/.test(name) && rhs.length > 0 && !rhs.includes("{")) {
                        if (rhs.includes(",") || rhs.startsWith("\"") || rhs.match(/[+\-*/()[\].<>=]/)) {
                            const assignLike = /^\w+$/.test(name) && !/^[A-Za-z_]\w*\s*,/.test(rhs);
                            if (assignLike && !rhs.includes(",")) {
                                out.push(`${name} = ${this.rewriteExpr(rhs)};`);
                            } else {
                                out.push(`${name}(${rhs});`);
                            }
                            continue;
                        }
                    }
                }
            }
            out.push(this.rewriteStatement(line));
        }
        return prelude.join("\n") + "\n" + out.join("\n");
    }

    rewriteExpr(expr) {
        const converted = this.convertColonCalls(expr);
        return converted
            .replace(/\blen\s*\(/g, "__len(")
            .replace(/\bstr\s*\(/g, "__str(")
            .replace(/\bnum\s*\(/g, "__num(")
            .replace(/\bupper\s*\(/g, "__upper(")
            .replace(/\blower\s*\(/g, "__lower(");
    }

    convertColonCalls(expr) {
        if (!expr || expr.includes("{")) return expr;
        const idx = expr.indexOf(":");
        if (idx === -1) return expr;
        const name = expr.slice(0, idx).trim();
        if (!/^[A-Za-z_]\w*$/.test(name)) return expr;
        const args = expr.slice(idx + 1).trim();
        if (!args) return `${name}()`;
        return `${name}(${args})`;
    }

    rewriteStatement(line) {
        if (line.endsWith("{") || line.endsWith("}") || line.endsWith(";") || line.endsWith(",")) return line;
        if (/^["'][^"']+["']\s*:/.test(line)) return line;
        return `${this.rewriteExpr(line)};`;
    }
}
