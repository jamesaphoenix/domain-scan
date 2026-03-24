package com.example

object AppConfig {
  val defaultPort: Int = 8080
  val defaultHost: String = "localhost"

  def getConfig(): Map[String, String] = {
    Map("port" -> defaultPort.toString, "host" -> defaultHost)
  }
}

object UserService {
  def apply(repo: UserRepository): UserService = new UserService(repo)

  def default(): UserService = new UserService(new InMemoryUserRepository)
}

object Main {
  def main(args: Array[String]): Unit = {
    val service = UserService.default()
    println("Started")
  }
}
