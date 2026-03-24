package main

type UserDTO struct {
	ID    string `json:"id"`
	Name  string `json:"name"`
	Email string `json:"email,omitempty"`
	Age   *int   `json:"age,omitempty"`
}

type CreateUserRequest struct {
	Name  string `json:"name" db:"name"`
	Email string `json:"email" db:"email"`
}

type InternalConfig struct {
	Host string
	Port int
}
