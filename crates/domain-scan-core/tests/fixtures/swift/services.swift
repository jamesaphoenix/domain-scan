import Foundation

// Service class with naming convention
@MainActor
class UserService {
    func fetchUsers() async throws -> [String] {
        return []
    }

    func createUser(name: String) async throws {
    }

    func deleteUser(id: String) async throws {
    }
}

// Controller class
@objc
class APIController {
    func handleRequest() {
    }

    func handleResponse() {
    }
}

// Repository class
class UserRepository {
    func findById(id: String) -> String? {
        return nil
    }

    func save(user: String) {
    }
}
