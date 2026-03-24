// types.ts - Defines interfaces that are implemented in other files

export interface EventHandler {
  handle(event: Event): void;
  onError(error: Error): void;
  cleanup(): void;
}

export interface Repository<T> {
  find(id: string): Promise<T>;
  save(entity: T): Promise<void>;
  delete(id: string): Promise<boolean>;
}

export interface Serializable {
  serialize(): string;
  deserialize(data: string): void;
}
