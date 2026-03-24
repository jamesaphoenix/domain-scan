import { Injectable } from '@nestjs/common';

@Injectable()
export class UserService {
    findAll(): Promise<any[]> {
        return Promise.resolve([]);
    }
}
