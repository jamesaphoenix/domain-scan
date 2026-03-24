// Test fixture: TypeScript schemas (Zod, Effect Schema, Drizzle)

import { z } from 'zod';
import { Schema } from '@effect/schema';
import { pgTable, serial, varchar, integer } from 'drizzle-orm/pg-core';

// Zod schema
export const UserSchema = z.object({
  name: z.string(),
  email: z.string().email(),
  age: z.number().optional(),
});

// Effect Schema
export const ProductSchema = Schema.Struct({
  id: Schema.String,
  title: Schema.String,
  price: Schema.Number,
});

// Drizzle table
export const users = pgTable('users', {
  id: serial('id').primaryKey(),
  name: varchar('name', { length: 255 }),
  email: varchar('email', { length: 255 }).unique(),
  age: integer('age'),
});
