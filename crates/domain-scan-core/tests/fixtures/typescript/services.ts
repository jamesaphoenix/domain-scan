// Test fixture: TypeScript services (NestJS-style)

@Controller('/users')
export class UserController {
  constructor(private readonly userService: UserService) {}

  @Get('/')
  async findAll(): Promise<User[]> {
    return this.userService.findAll();
  }

  @Get('/:id')
  async findOne(id: string): Promise<User> {
    return this.userService.findById(id);
  }

  @Post('/')
  async create(data: CreateUserDto): Promise<User> {
    return this.userService.create(data);
  }

  @Delete('/:id')
  async remove(id: string): Promise<void> {
    return this.userService.remove(id);
  }
}

@Injectable()
export class AuthService {
  constructor(
    private readonly jwtService: JwtService,
    private readonly userRepo: UserRepository,
  ) {}

  async validateToken(token: string): Promise<User | null> {
    return null;
  }
}
