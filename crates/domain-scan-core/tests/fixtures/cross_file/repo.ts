// repo.ts - Implements Repository from types.ts

import { Repository, Serializable } from "./types";

export class UserRepo implements Repository<User> {
  async find(id: string): Promise<User> {
    return {} as User;
  }

  async save(entity: User): Promise<void> {
    // save user
  }

  async delete(id: string): Promise<boolean> {
    return true;
  }
}

interface User {
  id: string;
  name: string;
}
