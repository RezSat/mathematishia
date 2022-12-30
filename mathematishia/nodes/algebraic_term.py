import re
from .base import Node
from .number import NumberNode

class AlgebraicTermNode(Node):
    def __init__(self, left, right):
        left, right = self.__check__(left, right)
        super().__init__('algebraic-number', left=left,right=right)

    def __repr__(self):
        if str(self.left) == '1':
            return f"{self.right}"
        return f"{self.left}{self.right}"

    def __check__(self, left, right):
        pattern = r'(\d*\.\d+|\d+|[a-zA-Z]+)'
        matches = re.findall(pattern, right)
        numbers = []
        symbols = []

        for match in matches:
            if match.isdigit():
                numbers.append(str(match))
            else:
                symbols.append(match)
        # things yet to handle 2x2z, 2x3
        if len(numbers) > 0:
            return NumberNode(numbers[0]), symbols[0]
        else:
            return left, right

