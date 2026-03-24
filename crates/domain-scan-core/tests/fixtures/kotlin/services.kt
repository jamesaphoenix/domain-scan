package com.example

import org.springframework.web.bind.annotation.*
import org.springframework.stereotype.Service

@RestController
@RequestMapping("/api/users")
class UserController(private val userService: UserService) {

    @GetMapping("/{id}")
    fun getUser(@PathVariable id: String): User? {
        return userService.getUser(id)
    }

    @PostMapping
    fun createUser(@RequestBody request: CreateUserRequest): User {
        return userService.createUser(request)
    }
}

@Service
class OrderService {
    fun processOrder(orderId: String): Order? {
        return null
    }
}
