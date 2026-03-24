require 'json'
require 'net/http'
require_relative 'lib/user_service'
require_relative 'lib/order_service'

include Comparable
extend ActiveModel::Naming
