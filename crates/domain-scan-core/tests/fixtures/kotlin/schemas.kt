package com.example

data class User(
    val id: String,
    val name: String,
    val email: String,
    val age: Int
)

data class CreateUserRequest(
    val name: String,
    val email: String,
    val password: String
)

data class OrderItem(
    val productId: String,
    val quantity: Int,
    val price: Double
)

class NotADataClass(val value: String)
