package com.example;

import org.springframework.web.bind.annotation.*;
import org.springframework.stereotype.Service;

@RestController
@RequestMapping("/api/users")
public class UserController {
    private final UserService userService;

    public UserController(UserService userService) {
        this.userService = userService;
    }

    @GetMapping("/{id}")
    public User getUser(@PathVariable String id) {
        return userService.getUser(id);
    }

    @PostMapping
    public User createUser(@RequestBody CreateUserRequest request) {
        return userService.createUser(request);
    }
}

@Service
public class OrderService {
    public Order processOrder(String orderId) {
        return null;
    }
}
