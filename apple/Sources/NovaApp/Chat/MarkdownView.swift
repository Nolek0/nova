import SwiftUI
import AppKit

/// Rendu Markdown par blocs : titres, paragraphes, listes, citations, séparateurs,
/// tableaux simples et blocs de code colorés. Le style en ligne (gras, italique,
/// code, liens) est géré par `AttributedString`.
struct MarkdownView: View {
    let text: String
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ForEach(Array(MarkdownBlock.parse(text).enumerated()), id: \.offset) { _, block in
                view(for: block)
            }
        }
        .textSelection(.enabled)
    }

    @ViewBuilder
    private func view(for block: MarkdownBlock) -> some View {
        switch block {
        case .heading(let level, let t):
            Text(inline(t))
                .font(.system(size: level == 1 ? 20 : level == 2 ? 17 : 15, weight: .semibold))
                .foregroundStyle(Theme.text)
                .padding(.top, level <= 2 ? 6 : 2)
        case .paragraph(let t):
            Text(inline(t)).font(.system(size: 14)).foregroundStyle(Theme.text).lineSpacing(3)
                .fixedSize(horizontal: false, vertical: true)
        case .small(let t):
            Text(inline(t)).font(.system(size: 11)).foregroundStyle(Theme.textTertiary)
        case .list(let items, let ordered):
            VStack(alignment: .leading, spacing: 5) {
                ForEach(Array(items.enumerated()), id: \.offset) { i, item in
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(ordered ? "\(i + 1)." : "•").font(.system(size: 14, weight: ordered ? .medium : .bold))
                            .foregroundStyle(ordered ? Theme.textSecondary : Theme.accent)
                            .frame(minWidth: 14, alignment: .trailing)
                        Text(inline(item.text)).font(.system(size: 14)).foregroundStyle(Theme.text).lineSpacing(3)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .padding(.leading, CGFloat(item.indent) * 18)
                }
            }
        case .quote(let t):
            HStack(spacing: 10) {
                RoundedRectangle(cornerRadius: 2).fill(Theme.violet.opacity(0.6)).frame(width: 3)
                Text(inline(t)).font(.system(size: 14)).foregroundStyle(Theme.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        case .rule:
            Rectangle().fill(Theme.border).frame(height: 1).padding(.vertical, 4)
        case .code(let lang, let code):
            CodeBlockView(language: lang, code: code)
        case .table(let rows):
            TableBlockView(rows: rows)
        }
    }

    private func inline(_ s: String) -> AttributedString {
        let opts = AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace, failurePolicy: .returnPartiallyParsedIfPossible)
        var a = (try? AttributedString(markdown: s, options: opts)) ?? AttributedString(s)
        for run in a.runs where run.inlinePresentationIntent?.contains(.code) == true {
            a[run.range].font = .system(size: 13, design: .monospaced)
            a[run.range].backgroundColor = Theme.codeBackground
            a[run.range].foregroundColor = Theme.cyan
        }
        for run in a.runs where run.link != nil {
            a[run.range].foregroundColor = Theme.accent
            a[run.range].underlineStyle = .single
        }
        return a
    }
}

enum MarkdownBlock: Equatable {
    struct Item: Equatable { var text: String; var indent: Int }
    case heading(Int, String)
    case paragraph(String)
    case small(String)
    case list([Item], ordered: Bool)
    case quote(String)
    case rule
    case code(String, String)
    case table([[String]])

    static func parse(_ text: String) -> [MarkdownBlock] {
        var blocks: [MarkdownBlock] = []
        var paragraph: [String] = []
        var list: [Item] = []
        var listOrdered = false
        var lines = text.components(separatedBy: "\n")[...]

        func flushParagraph() {
            if !paragraph.isEmpty { blocks.append(.paragraph(paragraph.joined(separator: "\n"))); paragraph = [] }
        }
        func flushList() {
            if !list.isEmpty { blocks.append(.list(list, ordered: listOrdered)); list = [] }
        }
        func flush() { flushParagraph(); flushList() }

        while let raw = lines.popFirst() {
            let line = raw.trimmingCharacters(in: .whitespaces)
            // Bloc de code (y compris non terminé pendant le streaming)
            if line.hasPrefix("```") {
                flush()
                let lang = String(line.dropFirst(3)).trimmingCharacters(in: .whitespaces)
                var code: [String] = []
                while let next = lines.popFirst() {
                    if next.trimmingCharacters(in: .whitespaces).hasPrefix("```") { break }
                    code.append(next)
                }
                blocks.append(.code(lang, code.joined(separator: "\n")))
                continue
            }
            if line.isEmpty { flush(); continue }
            if line.hasPrefix("<sub>") {
                flush()
                blocks.append(.small(line.replacingOccurrences(of: "<sub>", with: "").replacingOccurrences(of: "</sub>", with: "")))
                continue
            }
            if let h = heading(line) { flush(); blocks.append(h); continue }
            if line == "---" || line == "***" || line == "___" { flush(); blocks.append(.rule); continue }
            if line.hasPrefix(">") {
                flush()
                blocks.append(.quote(String(line.dropFirst()).trimmingCharacters(in: .whitespaces)))
                continue
            }
            if line.hasPrefix("|"), line.hasSuffix("|") || line.filter({ $0 == "|" }).count >= 2 {
                flush()
                var rows = [cells(line)]
                while let next = lines.first, next.trimmingCharacters(in: .whitespaces).hasPrefix("|") {
                    lines.removeFirst()
                    let r = cells(next)
                    if !r.allSatisfy({ $0.allSatisfy { "-: ".contains($0) } }) { rows.append(r) }
                }
                blocks.append(.table(rows))
                continue
            }
            if let item = listItem(raw) {
                flushParagraph()
                if !list.isEmpty && item.ordered != listOrdered { flushList() }
                listOrdered = item.ordered
                list.append(Item(text: item.text, indent: item.indent))
                continue
            }
            if !list.isEmpty, raw.hasPrefix("  ") {
                list[list.count - 1].text += " " + line
                continue
            }
            flushList()
            paragraph.append(line)
        }
        flush()
        return blocks
    }

    private static func heading(_ line: String) -> MarkdownBlock? {
        let hashes = line.prefix { $0 == "#" }.count
        guard (1...6).contains(hashes), line.dropFirst(hashes).first == " " else { return nil }
        return .heading(hashes, String(line.dropFirst(hashes + 1)))
    }

    private static func listItem(_ raw: String) -> (text: String, ordered: Bool, indent: Int)? {
        let indent = raw.prefix { $0 == " " }.count / 2
        let line = raw.trimmingCharacters(in: .whitespaces)
        for marker in ["- ", "* ", "+ "] where line.hasPrefix(marker) {
            return (String(line.dropFirst(2)), false, indent)
        }
        if let dot = line.firstIndex(where: { $0 == "." || $0 == ")" }), line[..<dot].allSatisfy(\.isNumber), !line[..<dot].isEmpty,
           line.index(after: dot) < line.endIndex, line[line.index(after: dot)] == " " {
            return (String(line[line.index(dot, offsetBy: 2)...]), true, indent)
        }
        return nil
    }

    private static func cells(_ line: String) -> [String] {
        var s = line.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("|") { s.removeFirst() }
        if s.hasSuffix("|") { s.removeLast() }
        return s.components(separatedBy: "|").map { $0.trimmingCharacters(in: .whitespaces) }
    }
}

struct TableBlockView: View {
    let rows: [[String]]
    var body: some View {
        let cols = rows.map(\.count).max() ?? 0
        Grid(alignment: .leading, horizontalSpacing: 0, verticalSpacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { r, row in
                GridRow {
                    ForEach(0..<cols, id: \.self) { c in
                        Text((try? AttributedString(markdown: c < row.count ? row[c] : "")) ?? "")
                            .font(.system(size: 13, weight: r == 0 ? .semibold : .regular))
                            .foregroundStyle(Theme.text)
                            .padding(.horizontal, 10).padding(.vertical, 6)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(r == 0 ? Theme.surfaceRaised : .clear)
                    }
                }
                if r < rows.count - 1 { Divider().overlay(Theme.border) }
            }
        }
        .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(Theme.border))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

struct CodeBlockView: View {
    let language: String
    let code: String
    @State private var copied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(language.isEmpty ? "code" : language.lowercased())
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundStyle(Theme.textTertiary)
                Spacer()
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(code, forType: .string)
                    copied = true
                    DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { copied = false }
                } label: {
                    Label(copied ? "Copié" : "Copier", systemImage: copied ? "checkmark" : "doc.on.doc")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(copied ? Theme.success : Theme.textSecondary)
                }
                .buttonStyle(GhostButtonStyle(padding: 4))
            }
            .padding(.leading, 12).padding(.trailing, 6).padding(.vertical, 4)
            .background(Theme.surfaceRaised)

            ScrollView(.horizontal, showsIndicators: false) {
                Text(SyntaxHighlighter.highlight(code, language: language))
                    .font(.system(size: 12.5, design: .monospaced))
                    .lineSpacing(2.5)
                    .textSelection(.enabled)
                    .padding(12)
            }
        }
        .background(Theme.codeBackground)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.border))
    }
}

/// Coloration syntaxique légère, commune aux langages courants.
enum SyntaxHighlighter {
    static let keywords: Set<String> = [
        // Commun
        "if", "else", "for", "while", "do", "return", "break", "continue", "switch", "case", "default", "in", "true", "false", "null", "nil", "None", "True", "False",
        // Swift / Kotlin / Rust / Go
        "func", "let", "var", "struct", "class", "enum", "protocol", "extension", "import", "guard", "throws", "throw", "try", "catch", "async", "await",
        "public", "private", "static", "self", "Self", "init", "where", "fn", "mut", "impl", "trait", "use", "pub", "match", "package", "type", "interface", "go", "defer",
        // Python
        "def", "lambda", "from", "as", "with", "pass", "yield", "elif", "and", "or", "not", "is", "raise", "except", "finally", "global",
        // JS / TS / Java / C
        "function", "const", "new", "this", "typeof", "instanceof", "export", "extends", "implements", "void", "int", "float", "double", "char", "long", "bool", "boolean", "string",
        "undefined", "of", "final", "abstract", "override", "virtual", "include", "define", "echo", "fi", "then", "esac", "done",
        // SQL
        "SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE", "CREATE", "TABLE", "JOIN", "ON", "AND", "OR", "ORDER", "BY", "GROUP", "LIMIT", "VALUES", "SET",
    ]

    static func highlight(_ code: String, language: String) -> AttributedString {
        var out = AttributedString()
        let hashComments = ["python", "py", "bash", "sh", "zsh", "ruby", "rb", "yaml", "yml", "toml", "r", "perl"].contains(language.lowercased())
        let chars = Array(code)
        var i = 0
        func emit(_ s: String, _ color: Color) {
            var a = AttributedString(s)
            a.foregroundColor = color
            out += a
        }
        while i < chars.count {
            let c = chars[i]
            // Commentaires
            if (c == "/" && i + 1 < chars.count && chars[i + 1] == "/") || (hashComments && c == "#") || (c == "-" && i + 1 < chars.count && chars[i + 1] == "-" && language.lowercased() == "sql") {
                var j = i
                while j < chars.count, chars[j] != "\n" { j += 1 }
                emit(String(chars[i..<j]), Theme.codeComment); i = j; continue
            }
            if c == "/" && i + 1 < chars.count && chars[i + 1] == "*" {
                var j = i + 2
                while j + 1 < chars.count, !(chars[j] == "*" && chars[j + 1] == "/") { j += 1 }
                j = min(chars.count, j + 2)
                emit(String(chars[i..<j]), Theme.codeComment); i = j; continue
            }
            // Chaînes
            if c == "\"" || c == "'" || c == "`" {
                var j = i + 1
                while j < chars.count, chars[j] != c, chars[j] != "\n" || c == "`" {
                    if chars[j] == "\\" { j += 1 }
                    j += 1
                }
                j = min(chars.count, j + 1)
                emit(String(chars[i..<j]), Theme.codeString); i = j; continue
            }
            // Nombres
            if c.isNumber {
                var j = i
                while j < chars.count, chars[j].isNumber || chars[j] == "." || chars[j] == "_" || chars[j].isHexDigit || chars[j] == "x" { j += 1 }
                emit(String(chars[i..<j]), Theme.codeNumber); i = j; continue
            }
            // Identifiants
            if c.isLetter || c == "_" || c == "@" {
                var j = i
                while j < chars.count, chars[j].isLetter || chars[j].isNumber || chars[j] == "_" || (j == i && chars[j] == "@") { j += 1 }
                let word = String(chars[i..<j])
                if keywords.contains(word) { emit(word, Theme.codeKeyword) }
                else if word.first?.isUppercase == true { emit(word, Theme.codeType) }
                else { emit(word, Theme.text) }
                i = j; continue
            }
            emit(String(c), Theme.text.opacity(0.85))
            i += 1
        }
        return out
    }
}
