#include <string>
#include <vector>

// Simple class with methods and properties
class UserService {
public:
    UserService(const std::string& name) : name_(name) {}

    std::string getName() const { return name_; }
    void setName(const std::string& name) { name_ = name; }
    static int instanceCount() { return count_; }

private:
    std::string name_;
    static int count_;
};

// Abstract class
class BaseRepository {
public:
    virtual ~BaseRepository() = default;
    virtual void save() = 0;
    virtual void load() = 0;
    void log(const std::string& msg) {}
};

// Struct (default public)
struct Point {
    double x;
    double y;
    double z;
};

// Template class
template<typename T>
class Container {
public:
    void add(const T& item) { items_.push_back(item); }
    T get(int index) const { return items_[index]; }
    int size() const { return items_.size(); }

private:
    std::vector<T> items_;
};
