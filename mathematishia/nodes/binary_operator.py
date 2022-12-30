from .base import Node
from .number import NumberNode
from .unary_operator import UnaryOperatorNode
from .algebraic_term import AlgebraicTermNode


class BinaryOperatorNode(Node):
    def __init__(self, type, left, right):
        super().__init__(type, left=left, right=right)

    def __repr__(self):
        must = isinstance(self.right, AlgebraicTermNode)
        if isinstance(self.left, NumberNode) and must and self.type == "*":
            return f"{self.left}{self.right}"
        if isinstance(self.left, UnaryOperatorNode) and must and self.type == "*":
            if isinstance(self.left.value, NumberNode):
                return f"{self.left}{self.right}"
        return f"{self.left} {self.type} {self.right}"
