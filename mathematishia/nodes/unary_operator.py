from .base import Node

class UnaryOperatorNode(Node):
    def __init__(self, type, child):
        super().__init__(type, value=child)

    def __repr__(self):
        return f"{self.type} {self.value}"
