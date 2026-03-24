// Test fixture: TypeScript classes

export class UserService implements IUserService {
  private readonly db: Database;

  constructor(db: Database) {
    this.db = db;
  }

  async getUser(id: string): Promise<User> {
    return this.db.findById(id);
  }

  async createUser(data: CreateUserDto): Promise<User> {
    return this.db.save(data);
  }

  static fromConfig(config: Config): UserService {
    return new UserService(new Database(config));
  }
}

abstract class BaseRepository<T> {
  protected abstract tableName: string;

  abstract findById(id: string): Promise<T | null>;

  async findAll(): Promise<T[]> {
    return [];
  }
}

class Logger {
  private level: string = 'info';

  log(message: string): void {
    // log implementation
  }

  protected formatMessage(message: string): string {
    return `[${this.level}] ${message}`;
  }
}
