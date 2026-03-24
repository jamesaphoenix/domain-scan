package main

type Reader interface {
	Read(p []byte) (n int, err error)
}

type Writer interface {
	Write(p []byte) (n int, err error)
}

type ReadWriter interface {
	Reader
	Writer
}

type UserService interface {
	GetUser(id string) (*User, error)
	CreateUser(name string, email string) (*User, error)
	DeleteUser(id string) error
}

type privateInterface interface {
	internalMethod()
}
