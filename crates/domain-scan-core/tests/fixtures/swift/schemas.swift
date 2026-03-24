import Foundation

// Codable struct
struct UserDTO: Codable {
    let id: String
    let name: String
    let email: String
    let age: Int
}

// Codable struct with optional fields
struct ProductResponse: Codable, Equatable {
    let id: String
    let title: String
    let description: String?
    let price: Double
}

// Codable class
class OrderModel: Codable {
    var orderId: String
    var items: [String]
    var total: Double
}

// Non-codable struct (should NOT be detected as schema)
struct InternalConfig {
    let maxRetries: Int
    let timeout: Double
}
