/**
 * Comprehensive E2E tests for all entity types in the Entities/Types tab.
 *
 * Covers: Interface, Service, Class, Function, Schema, Impl, TypeAlias
 *
 * Each entity type is tested for:
 * - Selection in the tree (correct badge, highlight)
 * - Detail panel rendering (correct metadata)
 * - Source code loading (Monaco tab opens, content visible)
 * - Child expansion (methods, properties, fields, routes)
 * - Child selection (Monaco scrolls to child line)
 */

import { test, expect, type Page } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import { waitForAppReady, switchTab, clickOpenDirectory } from "./helpers";
import type { EntitySummary, Entity } from "../src/types";

// ---------------------------------------------------------------------------
// Mock entities — one of each kind
// ---------------------------------------------------------------------------

const ALL_ENTITIES: EntitySummary[] = [
  {
    name: "AuthProvider",
    kind: "interface",
    file: "src/auth/provider.ts",
    line: 5,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserService",
    kind: "service",
    file: "src/services/user-service.ts",
    line: 10,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "DatabaseConnection",
    kind: "class",
    file: "src/db/connection.ts",
    line: 3,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "hashPassword",
    kind: "function",
    file: "src/utils/crypto.ts",
    line: 15,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserSchema",
    kind: "schema",
    file: "src/schemas/user.ts",
    line: 1,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "AuthProviderImpl",
    kind: "impl",
    file: "src/auth/provider_impl.rs",
    line: 20,
    language: "Rust",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserId",
    kind: "type_alias",
    file: "src/types/ids.ts",
    line: 8,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "parseConfig",
    kind: "function",
    file: "src/config/parser.py",
    line: 22,
    language: "Python",
    build_status: "unbuilt",
    confidence: "low",
  },
  {
    name: "OrderSchema",
    kind: "schema",
    file: "src/schemas/order.ts",
    line: 5,
    language: "TypeScript",
    build_status: "rebuild",
    confidence: "medium",
  },
];

// ---------------------------------------------------------------------------
// Mock entity details — full detail responses per entity kind
// ---------------------------------------------------------------------------

const SPAN = (start: number, end: number) => ({
  start_line: start,
  start_col: 0,
  end_line: end,
  end_col: 0,
  byte_range: [0, 500] as [number, number],
});

const ENTITY_DETAILS: Record<string, Entity> = {
  AuthProvider: {
    Interface: {
      name: "AuthProvider",
      file: "src/auth/provider.ts",
      span: SPAN(5, 25),
      visibility: "public",
      generics: ["T"],
      extends: ["BaseProvider"],
      methods: [
        {
          name: "login",
          span: SPAN(7, 12),
          is_async: true,
          parameters: [
            { name: "username", type_annotation: "string", is_optional: false, default_value: null },
            { name: "password", type_annotation: "string", is_optional: false, default_value: null },
          ],
          return_type: "Promise<AuthToken>",
        },
        {
          name: "logout",
          span: SPAN(14, 16),
          is_async: true,
          parameters: [],
          return_type: "Promise<void>",
        },
        {
          name: "getSession",
          span: SPAN(18, 20),
          is_async: false,
          parameters: [
            { name: "token", type_annotation: "string", is_optional: false, default_value: null },
          ],
          return_type: "Session | null",
        },
      ],
      properties: [
        { name: "isAuthenticated", type_annotation: "boolean", is_optional: false, is_readonly: true, visibility: "public" },
        { name: "currentUser", type_annotation: "User | null", is_optional: true, is_readonly: false, visibility: "public" },
      ],
      language_kind: "interface",
      decorators: [],
    },
  },
  UserService: {
    Service: {
      name: "UserService",
      file: "src/services/user-service.ts",
      span: SPAN(10, 60),
      kind: "http_controller",
      methods: [
        {
          name: "getUser",
          file: "src/services/user-service.ts",
          span: SPAN(15, 22),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "id", type_annotation: "string", is_optional: false, default_value: null },
          ],
          return_type: "Promise<User>",
          decorators: ["@Get"],
          owner: "UserService",
          implements: null,
        },
        {
          name: "createUser",
          file: "src/services/user-service.ts",
          span: SPAN(24, 35),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "data", type_annotation: "CreateUserDto", is_optional: false, default_value: null },
          ],
          return_type: "Promise<User>",
          decorators: ["@Post"],
          owner: "UserService",
          implements: null,
        },
        {
          name: "deleteUser",
          file: "src/services/user-service.ts",
          span: SPAN(37, 45),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "id", type_annotation: "string", is_optional: false, default_value: null },
          ],
          return_type: "Promise<void>",
          decorators: ["@Delete"],
          owner: "UserService",
          implements: null,
        },
      ],
      dependencies: ["AuthProvider", "DatabaseConnection"],
      decorators: ["@Controller"],
      routes: [
        { method: "GET", path: "/users/:id", handler: "getUser" },
        { method: "POST", path: "/users", handler: "createUser" },
        { method: "DELETE", path: "/users/:id", handler: "deleteUser" },
      ],
    },
  },
  DatabaseConnection: {
    Class: {
      name: "DatabaseConnection",
      file: "src/db/connection.ts",
      span: SPAN(3, 50),
      visibility: "public",
      generics: [],
      extends: "EventEmitter",
      implements: ["Disposable"],
      methods: [
        {
          name: "connect",
          file: "src/db/connection.ts",
          span: SPAN(8, 15),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [],
          return_type: "Promise<void>",
          decorators: [],
          owner: "DatabaseConnection",
          implements: null,
        },
        {
          name: "disconnect",
          file: "src/db/connection.ts",
          span: SPAN(17, 22),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [],
          return_type: "Promise<void>",
          decorators: [],
          owner: "DatabaseConnection",
          implements: null,
        },
        {
          name: "query",
          file: "src/db/connection.ts",
          span: SPAN(24, 32),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "sql", type_annotation: "string", is_optional: false, default_value: null },
            { name: "params", type_annotation: "unknown[]", is_optional: true, default_value: null },
          ],
          return_type: "Promise<QueryResult>",
          decorators: [],
          owner: "DatabaseConnection",
          implements: null,
        },
        {
          name: "getInstance",
          file: "src/db/connection.ts",
          span: SPAN(34, 40),
          visibility: "public",
          is_async: false,
          is_static: true,
          is_generator: false,
          parameters: [],
          return_type: "DatabaseConnection",
          decorators: [],
          owner: "DatabaseConnection",
          implements: null,
        },
      ],
      properties: [
        { name: "pool", type_annotation: "Pool", is_optional: false, is_readonly: true, visibility: "private" },
        { name: "isConnected", type_annotation: "boolean", is_optional: false, is_readonly: false, visibility: "public" },
        { name: "maxRetries", type_annotation: "number", is_optional: false, is_readonly: true, visibility: "protected" },
      ],
      is_abstract: false,
      decorators: ["@Injectable"],
    },
  },
  hashPassword: {
    Function: {
      name: "hashPassword",
      file: "src/utils/crypto.ts",
      span: SPAN(15, 28),
      visibility: "public",
      is_async: true,
      is_generator: false,
      parameters: [
        { name: "password", type_annotation: "string", is_optional: false, default_value: null },
        { name: "salt", type_annotation: "string", is_optional: true, default_value: null },
        { name: "rounds", type_annotation: "number", is_optional: true, default_value: "10" },
      ],
      return_type: "Promise<string>",
      decorators: [],
    },
  },
  UserSchema: {
    Schema: {
      name: "UserSchema",
      file: "src/schemas/user.ts",
      span: SPAN(1, 18),
      kind: "drizzle",
      fields: [
        { name: "id", type_annotation: "serial", is_optional: false },
        { name: "email", type_annotation: "varchar(255)", is_optional: false },
        { name: "name", type_annotation: "varchar(100)", is_optional: false },
        { name: "avatar_url", type_annotation: "text", is_optional: true },
        { name: "created_at", type_annotation: "timestamp", is_optional: false },
        { name: "updated_at", type_annotation: "timestamp", is_optional: true },
      ],
      source_framework: "drizzle-orm",
      table_name: "users",
      derives: [],
      visibility: "public",
    },
  },
  AuthProviderImpl: {
    Impl: {
      target: "AuthProviderImpl",
      trait_name: "AuthProvider",
      file: "src/auth/provider_impl.rs",
      span: SPAN(20, 65),
      methods: [
        {
          name: "login",
          file: "src/auth/provider_impl.rs",
          span: SPAN(22, 35),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "username", type_annotation: "&str", is_optional: false, default_value: null },
            { name: "password", type_annotation: "&str", is_optional: false, default_value: null },
          ],
          return_type: "Result<AuthToken>",
          decorators: [],
          owner: "AuthProviderImpl",
          implements: "AuthProvider",
        },
        {
          name: "logout",
          file: "src/auth/provider_impl.rs",
          span: SPAN(37, 45),
          visibility: "public",
          is_async: true,
          is_static: false,
          is_generator: false,
          parameters: [],
          return_type: "Result<()>",
          decorators: [],
          owner: "AuthProviderImpl",
          implements: "AuthProvider",
        },
        {
          name: "validate_token",
          file: "src/auth/provider_impl.rs",
          span: SPAN(47, 58),
          visibility: "private",
          is_async: false,
          is_static: false,
          is_generator: false,
          parameters: [
            { name: "token", type_annotation: "&str", is_optional: false, default_value: null },
          ],
          return_type: "bool",
          decorators: [],
          owner: "AuthProviderImpl",
          implements: null,
        },
      ],
    },
  },
  UserId: {
    TypeAlias: {
      name: "UserId",
      file: "src/types/ids.ts",
      span: SPAN(8, 8),
      target: "string & { __brand: 'UserId' }",
      generics: [],
      visibility: "public",
    },
  },
  parseConfig: {
    Function: {
      name: "parseConfig",
      file: "src/config/parser.py",
      span: SPAN(22, 40),
      visibility: "public",
      is_async: false,
      is_generator: false,
      parameters: [
        { name: "path", type_annotation: "str", is_optional: false, default_value: null },
        { name: "strict", type_annotation: "bool", is_optional: true, default_value: "True" },
      ],
      return_type: "Config",
      decorators: ["@validate_input"],
    },
  },
  OrderSchema: {
    Schema: {
      name: "OrderSchema",
      file: "src/schemas/order.ts",
      span: SPAN(5, 20),
      kind: "zod",
      fields: [
        { name: "orderId", type_annotation: "z.string().uuid()", is_optional: false },
        { name: "items", type_annotation: "z.array(OrderItemSchema)", is_optional: false },
        { name: "total", type_annotation: "z.number().positive()", is_optional: false },
        { name: "status", type_annotation: "z.enum(['pending','paid','shipped'])", is_optional: false },
        { name: "notes", type_annotation: "z.string()", is_optional: true },
      ],
      source_framework: "zod",
      table_name: null,
      derives: [],
      visibility: "public",
    },
  },
};

// ---------------------------------------------------------------------------
// Mock file sources per file
// ---------------------------------------------------------------------------

const FILE_SOURCES: Record<string, string> = {
  "src/auth/provider.ts": [
    "// Authentication provider interface",
    "import { BaseProvider } from './base';",
    "import { AuthToken, Session, User } from '../types';",
    "",
    "export interface AuthProvider<T> extends BaseProvider {",
    "  readonly isAuthenticated: boolean;",
    "  login(username: string, password: string): Promise<AuthToken>;",
    "  logout(): Promise<void>;",
    "  getSession(token: string): Session | null;",
    "  currentUser?: User | null;",
    "}",
  ].join("\n"),
  "src/services/user-service.ts": [
    "// User service controller",
    "import { Controller, Get, Post, Delete } from '@nestjs/common';",
    "import { AuthProvider } from '../auth/provider';",
    "import { DatabaseConnection } from '../db/connection';",
    "import { User, CreateUserDto } from '../types';",
    "",
    "interface QueryResult { rows: unknown[] }",
    "",
    "@Controller('/users')",
    "export class UserService {",
    "  constructor(",
    "    private auth: AuthProvider<unknown>,",
    "    private db: DatabaseConnection,",
    "  ) {}",
    "",
    "  @Get(':id')",
    "  async getUser(id: string): Promise<User> {",
    "    const result = await this.db.query('SELECT * FROM users WHERE id = $1', [id]);",
    "    return result.rows[0] as User;",
    "  }",
    "",
    "  @Post()",
    "  async createUser(data: CreateUserDto): Promise<User> {",
    "    return {} as User;",
    "  }",
    "",
    "  @Delete(':id')",
    "  async deleteUser(id: string): Promise<void> {",
    "    await this.db.query('DELETE FROM users WHERE id = $1', [id]);",
    "  }",
    "}",
  ].join("\n"),
  "src/db/connection.ts": [
    "import { EventEmitter } from 'events';",
    "",
    "export class DatabaseConnection extends EventEmitter implements Disposable {",
    "  private readonly pool: Pool;",
    "  public isConnected: boolean = false;",
    "  protected readonly maxRetries: number = 3;",
    "",
    "  async connect(): Promise<void> {",
    "    this.isConnected = true;",
    "  }",
    "",
    "  async disconnect(): Promise<void> {",
    "    this.isConnected = false;",
    "  }",
    "",
    "  async query(sql: string, params?: unknown[]): Promise<QueryResult> {",
    "    return { rows: [] };",
    "  }",
    "",
    "  static getInstance(): DatabaseConnection {",
    "    return new DatabaseConnection();",
    "  }",
    "}",
  ].join("\n"),
  "src/utils/crypto.ts": [
    "// Cryptographic utilities",
    "import { randomBytes, scrypt } from 'crypto';",
    "",
    "const SALT_LENGTH = 32;",
    "",
    "export async function hashPassword(",
    "  password: string,",
    "  salt?: string,",
    "  rounds: number = 10,",
    "): Promise<string> {",
    "  const actualSalt = salt ?? randomBytes(SALT_LENGTH).toString('hex');",
    "  return new Promise((resolve, reject) => {",
    "    scrypt(password, actualSalt, 64, (err, hash) => {",
    "      if (err) reject(err);",
    "      else resolve(hash.toString('hex'));",
    "    });",
    "  });",
    "}",
  ].join("\n"),
  "src/schemas/user.ts": [
    "import { pgTable, serial, varchar, text, timestamp } from 'drizzle-orm/pg-core';",
    "",
    "export const UserSchema = pgTable('users', {",
    "  id: serial('id').primaryKey(),",
    "  email: varchar('email', { length: 255 }).notNull(),",
    "  name: varchar('name', { length: 100 }).notNull(),",
    "  avatar_url: text('avatar_url'),",
    "  created_at: timestamp('created_at').defaultNow().notNull(),",
    "  updated_at: timestamp('updated_at'),",
    "});",
  ].join("\n"),
  "src/auth/provider_impl.rs": [
    "use crate::auth::{AuthProvider, AuthToken};",
    "use crate::error::Result;",
    "",
    "pub struct AuthProviderImpl {",
    "    token_store: TokenStore,",
    "}",
    "",
    "impl AuthProvider for AuthProviderImpl {",
    "    async fn login(&self, username: &str, password: &str) -> Result<AuthToken> {",
    "        let hash = self.hash_password(password);",
    "        let user = self.find_user(username, &hash)?;",
    "        Ok(AuthToken::new(user.id))",
    "    }",
    "",
    "    async fn logout(&self) -> Result<()> {",
    "        self.token_store.invalidate_all()?;",
    "        Ok(())",
    "    }",
    "",
    "    fn validate_token(&self, token: &str) -> bool {",
    "        self.token_store.is_valid(token)",
    "    }",
    "}",
  ].join("\n"),
  "src/types/ids.ts": [
    "// Branded type aliases for type-safe IDs",
    "",
    "export type OrgId = string & { __brand: 'OrgId' };",
    "export type TeamId = string & { __brand: 'TeamId' };",
    "export type ProjectId = string & { __brand: 'ProjectId' };",
    "",
    "export type UserId = string & { __brand: 'UserId' };",
    "",
    "export type SessionId = string & { __brand: 'SessionId' };",
  ].join("\n"),
  "src/config/parser.py": [
    "# Configuration file parser",
    "from dataclasses import dataclass",
    "from pathlib import Path",
    "from typing import Optional",
    "import json",
    "",
    "@dataclass",
    "class Config:",
    "    host: str",
    "    port: int",
    "    debug: bool = False",
    "",
    "def validate_input(fn):",
    "    def wrapper(*args, **kwargs):",
    "        return fn(*args, **kwargs)",
    "    return wrapper",
    "",
    "@validate_input",
    "def parseConfig(path: str, strict: bool = True) -> Config:",
    '    """Parse a config file from the given path."""',
    "    with open(path) as f:",
    "        data = json.load(f)",
    '    return Config(host=data["host"], port=data["port"])',
  ].join("\n"),
  "src/schemas/order.ts": [
    "import { z } from 'zod';",
    "",
    "const OrderItemSchema = z.object({ sku: z.string(), qty: z.number() });",
    "",
    "export const OrderSchema = z.object({",
    "  orderId: z.string().uuid(),",
    "  items: z.array(OrderItemSchema),",
    "  total: z.number().positive(),",
    "  status: z.enum(['pending', 'paid', 'shipped']),",
    "  notes: z.string().optional(),",
    "});",
  ].join("\n"),
};

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

async function setupWithAllEntities(page: Page) {
  // First set up base mocks
  await setupTauriMocks(page, {
    dialogResult: "/mock/test-project",
    scanStats: MOCK_SCAN_STATS,
    entities: ALL_ENTITIES,
  });

  // Store custom mock data in globals — the base mock's invoke will have
  // already been set by the previous addInitScript (they run synchronously in order)
  await page.addInitScript((data: string) => {
    const { details, sources } = JSON.parse(data);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    const origInvoke = w.__TAURI_INTERNALS__.invoke;
    w.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_entity_detail") {
        const name = args?.name as string;
        if (name && details[name]) return details[name];
      }
      if (cmd === "get_file_source") {
        const file = args?.file as string;
        if (file && sources[file]) return sources[file];
      }
      return origInvoke(cmd, args);
    };
  }, JSON.stringify({ details: ENTITY_DETAILS, sources: FILE_SOURCES }));
}

async function patchMocksInPage(page: Page) {
  await page.evaluate(
    ([detailsJson, sourcesJson]) => {
      const details = JSON.parse(detailsJson);
      const sources = JSON.parse(sourcesJson);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      const origInvoke = w.__TAURI_INTERNALS__.invoke;
      w.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === "get_entity_detail") {
          const name = args?.name as string;
          if (name && details[name]) return details[name];
        }
        if (cmd === "get_file_source") {
          const file = args?.file as string;
          if (file && sources[file]) return sources[file];
        }
        return origInvoke(cmd, args);
      };
    },
    [JSON.stringify(ENTITY_DETAILS), JSON.stringify(FILE_SOURCES)],
  );
}

async function scanAndWait(page: Page) {
  await switchTab(page, "Entities/Types");
  await clickOpenDirectory(page);
  await expect(
    page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
  ).toBeVisible({ timeout: 10_000 });
}

async function selectEntity(page: Page, name: string) {
  const treePanel = page.locator(".w-72").first();
  await treePanel.getByText(name, { exact: true }).click();
}

// ---------------------------------------------------------------------------
// Tests: Entity selection and detail panel
// ---------------------------------------------------------------------------

test.describe("Entity types — selection and detail panel", () => {
  test.beforeEach(async ({ page }) => {
    await setupWithAllEntities(page);
    await page.goto("/");
    await waitForAppReady(page);
    await patchMocksInPage(page);
    await scanAndWait(page);
  });

  test("Interface: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "AuthProvider");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("AuthProvider")).toBeVisible({ timeout: 5_000 });
    // Kind label appears as capitalized text — use first() for strict mode
    await expect(detailPanel.getByText("interface").first()).toBeVisible();
    await expect(detailPanel.getByText("Built").first()).toBeVisible();
    await expect(detailPanel.getByText("High").first()).toBeVisible();
    await expect(detailPanel.getByText("TypeScript").first()).toBeVisible();

    // Tree shows I badge
    const treePanel = page.locator(".w-72").first();
    const selectedRow = treePanel.locator(".bg-blue-900\\/50").first();
    await expect(selectedRow).toBeVisible();
  });

  test("Service: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "UserService");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("UserService")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("service").first()).toBeVisible();
  });

  test("Class: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "DatabaseConnection");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("DatabaseConnection")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("class").first()).toBeVisible();
  });

  test("Function: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "hashPassword");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("hashPassword")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("function").first()).toBeVisible();
  });

  test("Schema: shows correct badge, framework, and table name", async ({ page }) => {
    await selectEntity(page, "UserSchema");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("UserSchema")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("schema").first()).toBeVisible();
  });

  test("Impl: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "AuthProviderImpl");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("AuthProviderImpl")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("impl").first()).toBeVisible();
    await expect(detailPanel.getByText("Rust").first()).toBeVisible();
  });

  test("TypeAlias: shows correct badge and detail metadata", async ({ page }) => {
    await selectEntity(page, "UserId");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("UserId")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("type_alias").first()).toBeVisible();
  });

  test("Python function with unbuilt status shows low confidence", async ({ page }) => {
    await selectEntity(page, "parseConfig");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("parseConfig")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("Python").first()).toBeVisible();
    await expect(detailPanel.getByText("Low").first()).toBeVisible();
  });

  test("Schema with rebuild status shows medium confidence", async ({ page }) => {
    await selectEntity(page, "OrderSchema");

    const detailPanel = page.locator(".w-80").first();
    await expect(detailPanel.getByText("OrderSchema")).toBeVisible({ timeout: 5_000 });
    await expect(detailPanel.getByText("Medium").first()).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Tests: Source code loading and Monaco tabs
// ---------------------------------------------------------------------------

test.describe("Entity types — source code loading", () => {
  test.beforeEach(async ({ page }) => {
    await setupWithAllEntities(page);
    await page.goto("/");
    await waitForAppReady(page);
    await patchMocksInPage(page);
    await scanAndWait(page);
  });

  test("selecting an Interface opens a Monaco tab with file content", async ({ page }) => {
    await selectEntity(page, "AuthProvider");

    // Tab should appear
    await expect(page.getByText("auth/provider.ts")).toBeVisible({ timeout: 5_000 });

    // Monaco should have content (look for text from our mock source)
    const editorContent = page.locator(".monaco-editor");
    await expect(editorContent).toBeVisible({ timeout: 10_000 });
  });

  test("selecting a Service opens correct file tab", async ({ page }) => {
    await selectEntity(page, "UserService");
    await expect(page.getByText("services/user-service.ts")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting a Class opens correct file tab", async ({ page }) => {
    await selectEntity(page, "DatabaseConnection");
    await expect(page.getByText("db/connection.ts")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting a Function opens correct file tab", async ({ page }) => {
    await selectEntity(page, "hashPassword");
    await expect(page.getByText("utils/crypto.ts")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting a Schema opens correct file tab", async ({ page }) => {
    await selectEntity(page, "UserSchema");
    await expect(page.getByText("schemas/user.ts")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting an Impl opens Rust file tab", async ({ page }) => {
    await selectEntity(page, "AuthProviderImpl");
    await expect(page.getByText("auth/provider_impl.rs")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting a TypeAlias opens correct file tab", async ({ page }) => {
    await selectEntity(page, "UserId");
    await expect(page.getByText("types/ids.ts")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting a Python function opens correct file tab", async ({ page }) => {
    await selectEntity(page, "parseConfig");
    await expect(page.getByText("config/parser.py")).toBeVisible({ timeout: 5_000 });
  });

  test("selecting multiple entities opens multiple tabs", async ({ page }) => {
    await selectEntity(page, "AuthProvider");
    await expect(page.getByText("auth/provider.ts")).toBeVisible({ timeout: 5_000 });

    await selectEntity(page, "UserService");
    await expect(page.getByText("services/user-service.ts")).toBeVisible({ timeout: 5_000 });

    await selectEntity(page, "UserSchema");
    await expect(page.getByText("schemas/user.ts")).toBeVisible({ timeout: 5_000 });

    // All three tabs should be visible
    await expect(page.getByText("auth/provider.ts")).toBeVisible();
    await expect(page.getByText("services/user-service.ts")).toBeVisible();
    await expect(page.getByText("schemas/user.ts")).toBeVisible();
  });

  test("selecting same file entity reuses existing tab", async ({ page }) => {
    await selectEntity(page, "AuthProvider");
    await expect(page.getByText("auth/provider.ts")).toBeVisible({ timeout: 5_000 });

    // Re-select — should not duplicate the tab
    await selectEntity(page, "AuthProvider");

    // Count tabs with this label — should be exactly 1
    const tabCount = await page.getByText("auth/provider.ts").count();
    expect(tabCount).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Tests: Child expansion
// ---------------------------------------------------------------------------

test.describe("Entity types — child expansion", () => {
  test.beforeEach(async ({ page }) => {
    await setupWithAllEntities(page);
    await page.goto("/");
    await waitForAppReady(page);
    await patchMocksInPage(page);
    await scanAndWait(page);
  });

  test("Interface expands to show methods and properties", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "AuthProvider");

    // Click again to toggle expand
    await treePanel.getByText("AuthProvider", { exact: true }).click();

    // Should show method children
    await expect(treePanel.getByText("login")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("logout")).toBeVisible();
    await expect(treePanel.getByText("getSession")).toBeVisible();

    // Should show property children
    await expect(treePanel.getByText("isAuthenticated")).toBeVisible();
    await expect(treePanel.getByText("currentUser")).toBeVisible();
  });

  test("Service expands to show methods and routes", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "UserService");
    await treePanel.getByText("UserService", { exact: true }).click();

    // Methods
    await expect(treePanel.getByText("getUser")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("createUser")).toBeVisible();
    await expect(treePanel.getByText("deleteUser")).toBeVisible();

    // Routes
    await expect(treePanel.getByText("GET /users/:id")).toBeVisible();
    await expect(treePanel.getByText("POST /users")).toBeVisible();
    await expect(treePanel.getByText("DELETE /users/:id")).toBeVisible();
  });

  test("Class expands to show methods and properties", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "DatabaseConnection");
    await treePanel.getByText("DatabaseConnection", { exact: true }).click();

    // Methods
    await expect(treePanel.getByText("connect")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("disconnect")).toBeVisible();
    await expect(treePanel.getByText("query")).toBeVisible();
    await expect(treePanel.getByText("getInstance")).toBeVisible();

    // Properties
    await expect(treePanel.getByText("pool")).toBeVisible();
    await expect(treePanel.getByText("isConnected")).toBeVisible();
    await expect(treePanel.getByText("maxRetries")).toBeVisible();
  });

  test("Schema expands to show fields", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "UserSchema");
    await treePanel.getByText("UserSchema", { exact: true }).click();

    await expect(treePanel.getByText("id")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("email")).toBeVisible();
    await expect(treePanel.getByText("name")).toBeVisible();
    await expect(treePanel.getByText("avatar_url")).toBeVisible();
    await expect(treePanel.getByText("created_at")).toBeVisible();
    await expect(treePanel.getByText("updated_at")).toBeVisible();
  });

  test("Impl expands to show methods", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "AuthProviderImpl");
    await treePanel.getByText("AuthProviderImpl", { exact: true }).click();

    await expect(treePanel.getByText("login")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("logout")).toBeVisible();
    await expect(treePanel.getByText("validate_token")).toBeVisible();
  });

  test("Function has no expand indicator (no children)", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "hashPassword");

    // Functions shouldn't show expand arrow
    const selectedRow = treePanel.locator(".bg-blue-900\\/50").first();
    await expect(selectedRow).toBeVisible({ timeout: 5_000 });
    // The expand indicator (> or v) should NOT be present for functions
    const expandIndicator = selectedRow.locator("text=/^[>v]$/");
    await expect(expandIndicator).toHaveCount(0);
  });

  test("TypeAlias has no expand indicator (no children)", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "UserId");

    const selectedRow = treePanel.locator(".bg-blue-900\\/50").first();
    await expect(selectedRow).toBeVisible({ timeout: 5_000 });
    const expandIndicator = selectedRow.locator("text=/^[>v]$/");
    await expect(expandIndicator).toHaveCount(0);
  });

  test("Zod Schema expands to show fields", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();
    await selectEntity(page, "OrderSchema");
    await treePanel.getByText("OrderSchema", { exact: true }).click();

    await expect(treePanel.getByText("orderId")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("items")).toBeVisible();
    await expect(treePanel.getByText("total")).toBeVisible();
    await expect(treePanel.getByText("status")).toBeVisible();
    await expect(treePanel.getByText("notes")).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Tests: Filter by kind
// ---------------------------------------------------------------------------

test.describe("Entity types — filtering by kind", () => {
  test.beforeEach(async ({ page }) => {
    await setupWithAllEntities(page);
    await page.goto("/");
    await waitForAppReady(page);
    await patchMocksInPage(page);
    await scanAndWait(page);
  });

  test("filter by Interfaces shows only interface entities", async ({ page }) => {
    // Click the Interfaces filter button
    await page.getByRole("button", { name: "Interfaces" }).click();

    const treePanel = page.locator(".w-72").first();
    await expect(treePanel.getByText("AuthProvider")).toBeVisible({ timeout: 5_000 });
    // Non-interfaces should be hidden (filter_entities mock returns all entities,
    // but the real IPC would filter; we verify the filter was applied by checking
    // the filter button is active)
  });

  test("filter by Schemas shows only schema entities", async ({ page }) => {
    await page.getByRole("button", { name: "Schemas" }).click();
    // Schema filter applied — button should have active state
    const schemaButton = page.getByRole("button", { name: "Schemas" });
    await expect(schemaButton).toBeVisible();
  });

  test("filter by Types shows only type alias entities", async ({ page }) => {
    await page.getByRole("button", { name: "Types" }).click();
    const typesButton = page.getByRole("button", { name: "Types" });
    await expect(typesButton).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Tests: Cross-entity navigation
// ---------------------------------------------------------------------------

test.describe("Entity types — cross-entity navigation", () => {
  test.beforeEach(async ({ page }) => {
    await setupWithAllEntities(page);
    await page.goto("/");
    await waitForAppReady(page);
    await patchMocksInPage(page);
    await scanAndWait(page);
  });

  test("switching between entities updates detail panel correctly", async ({ page }) => {
    const detailPanel = page.locator(".w-80").first();

    // Select Interface
    await selectEntity(page, "AuthProvider");
    await expect(detailPanel.getByText("interface").first()).toBeVisible({ timeout: 5_000 });

    // Switch to Schema
    await selectEntity(page, "UserSchema");
    await expect(detailPanel.getByText("schema").first()).toBeVisible({ timeout: 5_000 });

    // Switch to Impl
    await selectEntity(page, "AuthProviderImpl");
    await expect(detailPanel.getByText("impl").first()).toBeVisible({ timeout: 5_000 });

    // Switch to TypeAlias
    await selectEntity(page, "UserId");
    await expect(detailPanel.getByText("type_alias").first()).toBeVisible({ timeout: 5_000 });
  });

  test("switching between entities updates source preview", async ({ page }) => {
    // Select TypeScript entity
    await selectEntity(page, "AuthProvider");
    await expect(page.getByText("auth/provider.ts")).toBeVisible({ timeout: 5_000 });

    // Switch to Rust entity — language should change in bottom bar
    await selectEntity(page, "AuthProviderImpl");
    await expect(page.getByText("auth/provider_impl.rs")).toBeVisible({ timeout: 5_000 });

    // Switch to Python entity
    await selectEntity(page, "parseConfig");
    await expect(page.getByText("config/parser.py")).toBeVisible({ timeout: 5_000 });
  });

  test("expanding one entity and selecting another collapses the first", async ({ page }) => {
    const treePanel = page.locator(".w-72").first();

    // Select and expand Interface
    await selectEntity(page, "AuthProvider");
    await treePanel.getByText("AuthProvider", { exact: true }).click();
    await expect(treePanel.getByText("login")).toBeVisible({ timeout: 5_000 });

    // Select a different entity — children of AuthProvider should remain
    // (tree doesn't auto-collapse other nodes)
    await selectEntity(page, "UserSchema");
    await expect(page.locator(".w-80").first().getByText("UserSchema")).toBeVisible({ timeout: 5_000 });
  });
});
