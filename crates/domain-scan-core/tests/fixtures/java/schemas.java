package com.example;

import javax.persistence.Entity;
import javax.persistence.Id;
import javax.persistence.Table;

@Entity
@Table(name = "users")
public class User {
    @Id
    private String id;
    private String name;
    private String email;
    private int age;

    public String getId() { return id; }
    public String getName() { return name; }
    public String getEmail() { return email; }
}

public record UserDTO(String name, String email, int age) {}

public record CreateUserRequest(
    String name,
    String email,
    String password
) {}

public class OrderItem {
    private String productId;
    private int quantity;
    private double price;
}
