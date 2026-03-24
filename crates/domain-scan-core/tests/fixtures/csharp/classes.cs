using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace DomainScan.Tests
{
    public class UserService
    {
        private readonly IUserRepository _repository;
        private readonly INotificationService _notificationService;

        public UserService(IUserRepository repository, INotificationService notificationService)
        {
            _repository = repository;
            _notificationService = notificationService;
        }

        public async Task<User> GetUser(int id)
        {
            return await _repository.GetById(id);
        }

        public async Task CreateUser(string name, string email)
        {
            var user = new User { Name = name, Email = email };
            await _repository.Save(user);
            _notificationService.SendEmail(email, "Welcome", "Hello!");
        }

        private void ValidateUser(User user)
        {
            // validation logic
        }

        public static UserService Create()
        {
            return null;
        }
    }

    public abstract class BaseEntity
    {
        public int Id { get; set; }
        public DateTime CreatedAt { get; set; }
        public DateTime UpdatedAt { get; set; }
    }

    public class GenericHandler<T> where T : class
    {
        public void Handle(T item)
        {
            // handle
        }
    }

    internal class InternalHelper
    {
        public void DoWork()
        {
        }
    }
}
