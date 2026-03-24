using System;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace DomainScan.Tests
{
    // C# record (data transfer object)
    public record UserDto(string Name, string Email, int Age);

    // C# record with additional body
    public record OrderDto(int OrderId, string CustomerName)
    {
        public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    }

    // Entity Framework entity class
    [Table("Users")]
    public class UserEntity
    {
        [Key]
        public int Id { get; set; }

        public string Name { get; set; }
        public string Email { get; set; }
        public DateTime CreatedAt { get; set; }
    }

    // Another EF entity
    [Table("Orders")]
    public class OrderEntity
    {
        [Key]
        public int Id { get; set; }

        public int UserId { get; set; }
        public decimal Total { get; set; }
        public string Status { get; set; }
    }

    // Regular class (not a schema - should be filtered out)
    public class NotASchema
    {
        public void DoSomething()
        {
        }
    }
}
