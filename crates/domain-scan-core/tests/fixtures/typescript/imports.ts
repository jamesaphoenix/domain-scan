// Test fixture: TypeScript imports

import { UserService, UserDto } from './services/user';
import type { Config } from './config';
import * as utils from './utils';
import express from 'express';
import { readFile as read, writeFile } from 'fs/promises';
