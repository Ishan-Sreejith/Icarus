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
            "CoRe Syntax Transpilation"
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
            (v) => String(v).toLowerCase()
        );
        return out.length ? out.join("\n") : "Program executed successfully";
    }

    transpile(source) {
        const lines = source.replace(/\r\n/g, "\n").split("\n");
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
        return out.join("\n");
    }

    rewriteExpr(expr) {
        return expr
            .replace(/\blen\s*\(/g, "__len(")
            .replace(/\bstr\s*\(/g, "__str(")
            .replace(/\bnum\s*\(/g, "__num(")
            .replace(/\bupper\s*\(/g, "__upper(")
            .replace(/\blower\s*\(/g, "__lower(");
    }

    rewriteStatement(line) {
        if (line.endsWith("{") || line.endsWith("}") || line.endsWith(";") || line.endsWith(",")) return line;
        if (/^["'][^"']+["']\s*:/.test(line)) return line;
        return `${this.rewriteExpr(line)};`;
    }
}
