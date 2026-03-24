<?php

interface UserRepositoryInterface {
    public function findById(int $id): ?User;
    public function save(User $user): void;
    public function delete(int $id): void;
}

interface NotificationServiceInterface extends ServiceInterface {
    public function send(string $message): bool;
    public function isConnected(): bool;
}

interface CacheInterface {
    public function get(string $key): mixed;
    public function set(string $key, mixed $value, int $ttl = 3600): void;
    public function has(string $key): bool;
}
