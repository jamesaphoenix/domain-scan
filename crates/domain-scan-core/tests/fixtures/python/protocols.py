from typing import Protocol


class Readable(Protocol):
    def read(self, size: int) -> bytes:
        ...


class Writable(Protocol):
    def write(self, data: bytes) -> int:
        ...


class Repository(Protocol):
    def find_by_id(self, id: str) -> dict:
        ...

    def save(self, item: dict) -> None:
        ...

    def delete(self, id: str) -> bool:
        ...
