from .base import Node

class NumberNode(Node):
    def __init__(self, value):
        super().__init__("number", value=value)

    def __repr__(self):
        return f"{self.value}"
