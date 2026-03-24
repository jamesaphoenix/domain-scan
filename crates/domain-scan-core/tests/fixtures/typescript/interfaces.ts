// Test fixture: TypeScript interfaces

export interface IUserService {
  getUser(id: string): Promise<User>;
  createUser(data: CreateUserDto): Promise<User>;
  deleteUser(id: string): Promise<void>;
}

interface ILogger {
  log(message: string, level?: string): void;
  error(message: string, error: Error): void;
}

export interface IRepository<T> {
  findById(id: string): Promise<T | null>;
  findAll(): Promise<T[]>;
  save(entity: T): Promise<T>;
  delete(id: string): Promise<boolean>;
}

export interface IConfig {
  readonly apiUrl: string;
  readonly port: number;
  debug?: boolean;
}

interface IEventEmitter extends IDisposable {
  on(event: string, handler: Function): void;
  emit(event: string, ...args: any[]): void;
}
