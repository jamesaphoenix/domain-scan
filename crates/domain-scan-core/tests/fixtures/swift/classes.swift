import Foundation

// Regular class
class UserManager {
    var users: [String] = []
    let maxUsers: Int

    init(maxUsers: Int) {
        self.maxUsers = maxUsers
    }

    func addUser(name: String) {
        users.append(name)
    }

    static func createDefault() -> UserManager {
        return UserManager(maxUsers: 100)
    }

    private func validate(name: String) -> Bool {
        return !name.isEmpty
    }
}

// Struct
struct Point {
    let x: Double
    let y: Double

    func distanceTo(other: Point) -> Double {
        let dx = x - other.x
        let dy = y - other.y
        return (dx * dx + dy * dy).squareRoot()
    }
}

// Enum with methods
enum Direction {
    case north
    case south
    case east
    case west

    func opposite() -> Direction {
        switch self {
        case .north: return .south
        case .south: return .north
        case .east: return .west
        case .west: return .east
        }
    }
}

// Class with inheritance and protocol conformance
public class AdminManager: UserManager {
    var adminLevel: Int

    init(maxUsers: Int, adminLevel: Int) {
        self.adminLevel = adminLevel
        super.init(maxUsers: maxUsers)
    }

    override func addUser(name: String) {
        super.addUser(name: name)
    }
}
