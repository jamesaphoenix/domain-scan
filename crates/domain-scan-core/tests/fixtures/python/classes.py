class UserService:
    def __init__(self, db):
        self.db = db

    def get_user(self, user_id: str) -> dict:
        pass

    async def create_user(self, name: str, email: str) -> dict:
        pass

    def delete_user(self, user_id: str) -> bool:
        pass


class Config:
    host = "localhost"
    port = 8080

    def as_dict(self) -> dict:
        return {"host": self.host, "port": self.port}


class _InternalHelper:
    def process(self):
        pass
