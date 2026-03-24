from fastapi import FastAPI

app = FastAPI()


@app.get("/users/{user_id}")
async def get_user(user_id: str):
    return {"id": user_id}


@app.post("/users")
async def create_user(name: str, email: str):
    return {"name": name, "email": email}


@app.delete("/users/{user_id}")
async def delete_user(user_id: str):
    return {"deleted": True}


@app.put("/users/{user_id}")
async def update_user(user_id: str, name: str):
    return {"id": user_id, "name": name}
