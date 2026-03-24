package com.example

trait UserRepository {
  def findById(id: String): Option[User]
  def findAll(): List[User]
  def save(user: User): Unit
  def delete(id: String): Unit
}

trait Closeable {
  def close(): Unit
}

trait EventHandler[T] extends Closeable {
  def handle(event: T): Unit
  def canHandle(eventType: String): Boolean
}

private[example] trait InternalCache {
  def get(key: String): Option[Any]
}
