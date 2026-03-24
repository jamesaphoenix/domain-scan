<?php

abstract class BaseEntity {
    protected int $id;

    abstract public function validate(): bool;

    public function getId(): int {
        return $this->id;
    }
}

class UserService implements UserRepositoryInterface {
    private string $name;
    protected readonly string $email;

    public function __construct(string $name, string $email) {
        $this->name = $name;
        $this->email = $email;
    }

    public function findById(int $id): ?User {
        return null;
    }

    public function save(User $user): void {}

    public function delete(int $id): void {}

    public static function create(string $name): self {
        return new self($name, '');
    }
}

class OrderController extends BaseController {
    public function index(): Response {
        return new Response();
    }

    public function store(Request $request): Response {
        return new Response();
    }
}
