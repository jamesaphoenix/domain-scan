// Test fixture: TypeScript exports

export const API_VERSION = '1.0';

export function createApp(): App {
  return new App();
}

export class Router {
  handle(): void {}
}

export interface IPlugin {
  name: string;
  init(): void;
}

export type AppConfig = { debug: boolean };

export default class Application {
  start(): void {}
}

export { UserService, UserDto } from './services/user';
export { helper as utilHelper } from './utils';
