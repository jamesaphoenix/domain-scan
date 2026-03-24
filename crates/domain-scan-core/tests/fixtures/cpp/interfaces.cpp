// Abstract base classes (C++ "interfaces")

class IRepository {
public:
    virtual ~IRepository() = default;
    virtual void save(int id) = 0;
    virtual void remove(int id) = 0;
    virtual int count() = 0;
};

class INotificationService {
public:
    virtual ~INotificationService() = default;
    virtual void send(const char* message) = 0;
    virtual bool isConnected() = 0;
};

// Not an interface: has non-virtual methods
class ConcreteClass {
public:
    void doSomething() {}
    int getValue() { return 42; }
};

// Not an interface: no pure virtual methods, just virtual
class BaseClass {
public:
    virtual void something() {}
    virtual ~BaseClass() = default;
};
