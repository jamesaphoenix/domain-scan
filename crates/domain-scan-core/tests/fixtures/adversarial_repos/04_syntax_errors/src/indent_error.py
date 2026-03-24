# Deliberately broken: mixed indentation and missing colon
class BrokenClass:
    def method_one(self):
        return True
	def method_two(self)
        return False
    def method_three(self):
	    return None
