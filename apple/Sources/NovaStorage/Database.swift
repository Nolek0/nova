import Foundation
import SQLite3

public enum DatabaseError: Error, LocalizedError {
    case open(String)
    case prepare(String)
    case step(String)

    public var errorDescription: String? {
        switch self {
        case .open(let m): return "Impossible d'ouvrir la base de données : \(m)"
        case .prepare(let m): return "Requête SQL invalide : \(m)"
        case .step(let m): return "Erreur SQL : \(m)"
        }
    }
}

/// Valeur liée à une requête SQL.
public enum SQLValue {
    case int(Int64)
    case double(Double)
    case text(String)
    case null
}

/// Ligne de résultat, accessible par nom de colonne.
public struct Row {
    fileprivate var values: [String: SQLValue]

    public func string(_ key: String) -> String? {
        if case .text(let s) = values[key] { return s }
        return nil
    }
    public func int(_ key: String) -> Int64? {
        switch values[key] {
        case .int(let i): return i
        case .double(let d): return Int64(d)
        default: return nil
        }
    }
    public func double(_ key: String) -> Double? {
        switch values[key] {
        case .double(let d): return d
        case .int(let i): return Double(i)
        default: return nil
        }
    }
}

/// Mince surcouche au-dessus de la bibliothèque SQLite système (aucune dépendance externe).
/// Toutes les opérations passent par une file série : l'objet est utilisable depuis n'importe quel thread.
public final class Database: @unchecked Sendable {
    private var handle: OpaquePointer?
    private let queue = DispatchQueue(label: "nova.database")
    private static let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    public let url: URL?

    /// - Parameter url: emplacement du fichier ; `nil` crée une base en mémoire (tests).
    public init(url: URL?) throws {
        self.url = url
        let path = url?.path ?? ":memory:"
        if let dir = url?.deletingLastPathComponent() {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        guard sqlite3_open_v2(path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX, nil) == SQLITE_OK else {
            throw DatabaseError.open(String(cString: sqlite3_errmsg(handle)))
        }
        try execute("PRAGMA journal_mode = WAL;")
        try execute("PRAGMA foreign_keys = ON;")
    }

    deinit { sqlite3_close(handle) }

    public func execute(_ sql: String) throws {
        try queue.sync {
            var err: UnsafeMutablePointer<CChar>?
            if sqlite3_exec(handle, sql, nil, nil, &err) != SQLITE_OK {
                let message = err.map { String(cString: $0) } ?? "inconnue"
                sqlite3_free(err)
                throw DatabaseError.step(message)
            }
        }
    }

    @discardableResult
    public func run(_ sql: String, _ params: [SQLValue] = []) throws -> Int64 {
        try queue.sync {
            let stmt = try prepare(sql, params)
            defer { sqlite3_finalize(stmt) }
            let rc = sqlite3_step(stmt)
            guard rc == SQLITE_DONE || rc == SQLITE_ROW else {
                throw DatabaseError.step(String(cString: sqlite3_errmsg(handle)))
            }
            return sqlite3_last_insert_rowid(handle)
        }
    }

    public func query(_ sql: String, _ params: [SQLValue] = []) throws -> [Row] {
        try queue.sync {
            let stmt = try prepare(sql, params)
            defer { sqlite3_finalize(stmt) }
            var rows: [Row] = []
            while true {
                let rc = sqlite3_step(stmt)
                if rc == SQLITE_DONE { break }
                guard rc == SQLITE_ROW else {
                    throw DatabaseError.step(String(cString: sqlite3_errmsg(handle)))
                }
                var values: [String: SQLValue] = [:]
                for i in 0..<sqlite3_column_count(stmt) {
                    let name = String(cString: sqlite3_column_name(stmt, i))
                    switch sqlite3_column_type(stmt, i) {
                    case SQLITE_INTEGER: values[name] = .int(sqlite3_column_int64(stmt, i))
                    case SQLITE_FLOAT: values[name] = .double(sqlite3_column_double(stmt, i))
                    case SQLITE_TEXT: values[name] = .text(String(cString: sqlite3_column_text(stmt, i)))
                    default: values[name] = .null
                    }
                }
                rows.append(Row(values: values))
            }
            return rows
        }
    }

    /// Exécute plusieurs écritures de façon atomique.
    public func transaction(_ body: () throws -> Void) throws {
        try execute("BEGIN;")
        do {
            try body()
            try execute("COMMIT;")
        } catch {
            try? execute("ROLLBACK;")
            throw error
        }
    }

    private func prepare(_ sql: String, _ params: [SQLValue]) throws -> OpaquePointer? {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(handle, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw DatabaseError.prepare(String(cString: sqlite3_errmsg(handle)))
        }
        for (index, value) in params.enumerated() {
            let i = Int32(index + 1)
            switch value {
            case .int(let v): sqlite3_bind_int64(stmt, i, v)
            case .double(let v): sqlite3_bind_double(stmt, i, v)
            case .text(let v): sqlite3_bind_text(stmt, i, v, -1, Database.transient)
            case .null: sqlite3_bind_null(stmt, i)
            }
        }
        return stmt
    }
}

public extension Database {
    /// Emplacement standard : ~/Library/Application Support/NovaAI/nova.sqlite
    static func defaultURL() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return base.appendingPathComponent("NovaAI", isDirectory: true).appendingPathComponent("nova.sqlite")
    }
}
