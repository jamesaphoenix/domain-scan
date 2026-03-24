package main

type User struct {
	ID    string
	Name  string
	Email string
	Age   int
}

type Config struct {
	Host     string
	Port     int
	Database string
}

type internalState struct {
	counter int
	active  bool
}
