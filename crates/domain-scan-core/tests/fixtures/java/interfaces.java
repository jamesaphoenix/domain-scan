package com.example;

public interface UserRepository {
    User findById(String id);
    List<User> findAll();
    void save(User user);
    void delete(String id);
}

interface Closeable {
    void close();
}

public interface EventHandler<T> extends Closeable {
    void handle(T event);
    boolean canHandle(String eventType);
}

interface ReadOnly {
    Object get(String key);
}
