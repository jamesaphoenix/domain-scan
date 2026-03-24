package main

type UserRepo struct {
	db *Database
}

func (r *UserRepo) FindByID(id string) (*User, error) {
	return nil, nil
}

func (r *UserRepo) Save(user *User) error {
	return nil
}

func (r *UserRepo) Delete(id string) error {
	return nil
}

type Logger struct{}

func (l Logger) Info(msg string) {}
func (l Logger) Error(msg string, err error) {}
