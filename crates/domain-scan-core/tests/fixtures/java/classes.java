package com.example;

public class UserService {
    private final UserRepository repository;

    public UserService(UserRepository repository) {
        this.repository = repository;
    }

    public User getUser(String id) {
        return repository.findById(id);
    }

    public void createUser(String name, String email) {
        User user = new User(name, email);
        repository.save(user);
    }

    private void validateEmail(String email) {
        // validation logic
    }
}

public abstract class BaseEntity {
    protected String id;
    protected String createdAt;

    public abstract String getType();
}

class InternalHelper {
    static void doSomething() {}
}

public class Config<T> {
    private T value;

    public T getValue() {
        return value;
    }
}
