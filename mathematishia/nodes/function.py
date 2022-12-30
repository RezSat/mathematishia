from .base import Node

class FunctionNode(Node):
    def __init__(self, left, right):
        super().__init__('function', left=left, right=right)

    def __repr__(self):
        if '(' in str(self.right):
            return f"{self.left}{self.right}"
        return f"{self.left}({self.right})"