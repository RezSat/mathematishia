from .base import Node

class ParenthesisNode(Node):
    def __init__(self, child):
        super().__init__("parenthesis",value=child)

    def __repr__(self):
        return f"({self.value})"

