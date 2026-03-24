// Test fixture: TypeScript type aliases

export type UserId = string;

export type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };

type InternalConfig = {
  host: string;
  port: number;
};

export type UserRole = 'admin' | 'user' | 'guest';

export type AsyncHandler<T> = (req: Request) => Promise<T>;
