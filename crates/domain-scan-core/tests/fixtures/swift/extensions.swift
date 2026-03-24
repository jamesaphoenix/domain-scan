import Foundation

class MyClass {
    var name: String = ""

    func greet() -> String {
        return "Hello, \(name)"
    }
}

protocol Printable {
    func printDescription()
}

// Simple extension
extension MyClass {
    func extraMethod() -> Int {
        return 42
    }

    func anotherMethod() {
    }
}

// Extension with protocol conformance
extension MyClass: Printable {
    func printDescription() {
        print(name)
    }
}

// Extension on a struct
extension String {
    func reversed() -> String {
        return String(self.reversed())
    }
}
