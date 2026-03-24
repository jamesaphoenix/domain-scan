using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace DomainScan.Tests
{
    public interface IUserRepository
    {
        Task<User> GetById(int id);
        Task<List<User>> GetAll();
        Task Save(User user);
        void Delete(int id);
    }

    public interface INotificationService
    {
        void SendEmail(string to, string subject, string body);
        Task SendSmsAsync(string phoneNumber, string message);
    }

    public interface IRepository<T> where T : class
    {
        Task<T> FindById(int id);
        Task<IEnumerable<T>> FindAll();
        Task Add(T entity);
        Task Update(T entity);
        void Remove(T entity);
    }

    internal interface IInternalService
    {
        void DoWork();
    }
}
