from pydantic import BaseModel
from dataclasses import dataclass
from typing import Optional, TypedDict


class UserSchema(BaseModel):
    id: int
    name: str
    email: Optional[str] = None


class CreateUserRequest(BaseModel):
    name: str
    email: str
    age: Optional[int] = None


@dataclass
class UserDTO:
    id: int
    name: str
    email: str = ""


class UserDict(TypedDict):
    id: int
    name: str
    email: str
