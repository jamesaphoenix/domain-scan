import Foundation

// Basic protocol
protocol Drawable {
    func draw()
    var color: String { get set }
}

// Protocol with inheritance
public protocol Serializable: Codable {
    func serialize() -> Data
    func deserialize(from data: Data) -> Self
}

// Protocol with associated type
protocol Repository {
    associatedtype Entity
    func findById(id: String) -> Entity?
    func save(entity: Entity)
    func delete(entity: Entity)
}

// Protocol with generic constraint
protocol Comparable: Equatable {
    func compare(to other: Self) -> Int
}
