class UserService < BaseService
  def initialize(name, email)
    @name = name
    @email = email
  end

  def find_by_id(id)
    # find user
  end

  def save(user)
    # save user
  end

  def self.create(name)
    new(name, '')
  end
end

class OrderController < ApplicationController
  def index
    @orders = Order.all
  end

  def show(id)
    @order = Order.find(id)
  end

  def self.route_prefix
    '/orders'
  end
end

class SimpleModel
  def to_s
    "SimpleModel"
  end
end
