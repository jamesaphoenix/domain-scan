// POD structs (data transfer objects)

struct UserDto {
    int id;
    const char* name;
    const char* email;
};

struct OrderDto {
    int orderId;
    double amount;
    bool isPaid;
};

// This should NOT be a schema (has methods)
struct ActiveRecord {
    int id;
    void save() {}
    void destroy() {}
};
