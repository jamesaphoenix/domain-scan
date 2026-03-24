package com.example

class UserService(repository: UserRepository) {
  def getUser(id: String): Option[User] = {
    repository.findById(id)
  }

  def createUser(name: String, email: String): User = {
    val user = User(name, email)
    repository.save(user)
    user
  }

  private def validateEmail(email: String): Boolean = {
    email.contains("@")
  }
}

abstract class BaseEntity {
  def id: String
  def getType(): String
}

case class User(name: String, email: String, age: Int = 0)

case class CreateUserRequest(
  name: String,
  email: String,
  password: String
)

object UserService {
  def apply(repo: UserRepository): UserService = new UserService(repo)
}
