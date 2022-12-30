from sympy import *
import re
from io import BytesIO, TextIOWrapper

from mathematishia.parsing.tokens import *
from mathematishia.parsing._parser import Parser
from mathematishia.parsing.lexer import tokenizer
from mathematishia.nodes import *

def group(*choices): return '(' + '|'.join(choices) + ')'
def any(*choices): return group(*choices) + '*'
def maybe(*choices): return group(*choices) + '?'

Exponent = r'[eE][-+]?[0-9](?:_?[0-9])*'
Pointfloat = group(r'[0-9](?:_?[0-9])*\.(?:[0-9](?:_?[0-9])*)?',
                   r'\.[0-9](?:_?[0-9])*') + maybe(Exponent)
Expfloat = r'[0-9](?:_?[0-9])*' + Exponent
Floatnumber = group(Pointfloat, Expfloat)

def isFloat(number):
    if re.match(Floatnumber, number):
        return True
    return False

def create_ast(expression):

    # Tokenize the expression
    tokens = tokenizer(expression)
    # Parse the tokens into an AST
    parser = Parser(tokens)
    return parser.parse()

class Step:
    def __init__(self, description, step=None, substeps=[], result=None):
        self.description = description
        self.substeps = substeps or []
        self.result = result
        self.step = step
        self.latex_code = {
            'step': "{} {} {}".format('{', create_ast(self.step).node, '}'),
            'result': "{} {} {}".format('{', self.result, '}')
        }

    def __repr__(self):
        if self.substeps:
            return f"{self.description}\n\t{self.substeps}"
        else:
            return self.description

class Interpreter():
    steps_array = []
    still_solving_paran = False
    current_sub_step = []

    def add_result(self,res):
        x = self.steps_array.append(res)
        self.steps_array = list(dict.fromkeys(self.steps_array))

    def visit(self, node):
        self.ast_copy = node
        method_name = f'visit_{type(node).__name__}'
        method = getattr(self, method_name, self.no_visit_method)
        return method(node)
    
    def no_visit_method(self, node):
        raise Exception(f'No visit{type(node).__name__} method defined')
        
    def visit_NumberNode(self, node):
        if isFloat(node.value) == True:
            return Float(node.value)
        else:
            return Integer(node.value)

    def visit_ParenthesisNode(self, node):
        self.still_solving_paran = True
        #idx = len(self.steps_array)-1
        res = self.visit(node.value)
        #self.steps_array[idx] = self.steps_array[idx].format(res)
        text_step = DESCRIPTIONS['paran'].format(node.value, create_ast(str(res)).node)
        latex_step = "\\textrm{ Evaluate within parenthesis: }" + f"( {node.value} ) = {create_ast(str(res)).node}"
        step = Step(
            description = {'text': text_step, 'latex': latex_step},
            step = str(node.value),
            result = create_ast(str(res)).node,
            substeps = [i.__dict__ for i in self.current_sub_step]
            )
        self.still_solving_paran = False
        self.add_result(step)
        return res
    
    def visit_AlgebraicTermNode(self, node):
        coefficient = self.visit(node.left)
        sym = Symbol(node.right)
        return coefficient * sym
    
    def visit_BinaryOperatorNode(self, node):
        left = self.visit(node.left)
        right = self.visit(node.right)

        node.left = left
        node.right = right

        if node.type == "+":
            result = left + right
            create_ast(str(left)).node
            latex_step = "\\textrm{ Add }" + f"{create_ast(str(left)).node}" + "\\textrm{ to }" + f"{create_ast(str(right)).node} = {create_ast(str(result)).node}"
            text_step = DESCRIPTIONS['add'].format(create_ast(str(left)).node, create_ast(str(right)).node, create_ast(str(result)).node)
            step = Step(
            description = {'text': text_step, 'latex': latex_step},
            step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
            result = create_ast(str(result)).node
            )
        if node.type == "-":
            result = left - right
            latex_step = "\\textrm{ Subtract }" + f"{create_ast(str(left)).node}" + "\\textrm{ from }" + f"{create_ast(str(right)).node} = {create_ast(str(result)).node}"
            text_step = DESCRIPTIONS['sub'].format(create_ast(str(left)).node, create_ast(str(right)).node, create_ast(str(result)).node)
            step = Step(
            description = {'text': text_step, 'latex': latex_step},
            step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
            result = create_ast(str(result)).node
            )
        if node.type == "*":
            result = left * right
            latex_step = "\\textrm{ Multiply }" + f"{create_ast(str(left)).node}" + "\\textrm{ and }" + f"{create_ast(str(right)).node} = {create_ast(str(result)).node}"
            text_step = DESCRIPTIONS['mul'].format(create_ast(str(left)).node, create_ast(str(right)).node, create_ast(str(result)).node)
            step = Step(
            description = {'text': text_step, 'latex': latex_step},
            step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
            result = create_ast(str(result)).node
            )
        if node.type == "/":
            if str(right) == '0':
                step = DESCRIPTIONS['zero_division_eror']
                latex_step = "\\textrm{ Divide by Zero is Undefined. }"
                text_result = simplify(left/right)
                step = Step(
                description = {'text': text_step, 'latex': latex_step},
                step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
                result = create_ast(str(result)).node
                )
            else:
                result = left / right
                latex_step = "\\textrm{ Divide }" + f"{create_ast(str(left)).node}" + "\\textrm{ by }" + f"{create_ast(str(right)).node} = {create_ast(str(result)).node}"
                text_step = DESCRIPTIONS['div'].format(create_ast(str(left)).node, create_ast(str(right)).node, create_ast(str(result)).node)
                step = Step(
                description = {'text': text_step, 'latex': latex_step},
                step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
                result = create_ast(str(result)).node
                )
        if node.type == "^":
            result = left ** right
            latex_step = f"{create_ast(str(left)).node}" + "\\textrm{ to the power of }" + f"{create_ast(str(right)).node} = {create_ast(str(result)).node}"
            text_step = DESCRIPTIONS['exp'].format(create_ast(str(left)).node, create_ast(str(right)).node, create_ast(str(result)).node)
            step = Step(
            description = {'text': text_step, 'latex': latex_step},
            step = "{} {} {}".format(create_ast(str(left)).node, node.type, create_ast(str(right)).node),
            result = create_ast(str(result)).node
            )

        if self.still_solving_paran:
            self.current_sub_step.append(step)
        else:
            self.current_sub_step = []
            self.add_result(step)

        #self.string_ast = f"{self.string_ast.replace(str(node), str(result), 1)}"
        #self.add_(self.string_ast)
        return result
    
    def visit_UnaryOperatorNode(self, node):
        number = self.visit(node.value)
        node.value = number
        if node.type == '-':
            number = number * -1
        if isinstance(number, Mul):
            return number
        if isFloat(str(node.value)) == True:
            ret =  Float(number)
        else:
            ret =  Integer(number)

        #self.string_ast = f"{self.string_ast.replace(str(node), str(ret), 1)}"
        #self.add_(self.string_ast)

        return ret 
