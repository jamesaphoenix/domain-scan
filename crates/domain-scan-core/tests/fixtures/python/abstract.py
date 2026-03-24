from abc import ABC, abstractmethod


class BaseHandler(ABC):
    @abstractmethod
    def handle(self, event: dict) -> None:
        pass

    @abstractmethod
    def name(self) -> str:
        pass

    def log(self, message: str) -> None:
        print(message)


class BaseRepository(ABC):
    @abstractmethod
    def find(self, id: str):
        pass

    @abstractmethod
    def save(self, item):
        pass
