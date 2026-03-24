package com.example

class UserService(private val repository: UserRepository) {
    fun getUser(id: String): User? {
        return repository.findById(id)
    }

    fun createUser(name: String, email: String): User {
        val user = User(name, email)
        repository.save(user)
        return user
    }

    private fun validateEmail(email: String): Boolean {
        return email.contains("@")
    }
}

abstract class BaseEntity {
    abstract val id: String
    abstract fun getType(): String
}

open class Config<T>(val value: T) {
    fun getValue(): T = value
}

internal class InternalHelper {
    companion object {
        fun doSomething() {}
    }
}

object Singleton {
    fun instance(): Singleton = this
}
