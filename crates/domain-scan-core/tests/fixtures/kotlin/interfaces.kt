package com.example

interface UserRepository {
    fun findById(id: String): User?
    fun findAll(): List<User>
    fun save(user: User)
    fun delete(id: String)
}

interface Closeable {
    fun close()
}

interface EventHandler<T> : Closeable {
    fun handle(event: T)
    fun canHandle(eventType: String): Boolean
}

private interface InternalCache {
    fun get(key: String): Any?
}
