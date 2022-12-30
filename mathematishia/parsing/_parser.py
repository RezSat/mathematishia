from mathematishia.nodes import *
from mathematishia.parsing.tokens import *
from mathematishia.parsing.lexer import isFloat
from tokenize import TokenInfo
    
class ParseResult:
    def __init__(self):
        self.error = None
        self.node = None
        self.advance_count = 0

    def register_advancement(self):
        self.advance_count += 1

    def register(self, res):
        self.advance_count += res.advance_count
        if res.error: self.error = res.error
        return res.node

    def success(self, node):
        self.node = node
        return self

    def failure(self, error):
        if not self.error or self.advance_count == 0:
            self.error = error
        return self

class Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.tok_idx = -1
        self.advance()
        
    def advance(self):
        self.tok_idx += 1
        if self.tok_idx  < len(self.tokens):
            self.current_tok = self.tokens[self.tok_idx]
        return self.current_tok


    def peek_next(self):
        return self.tokens[self.tok_idx + 1]

    def peek(self):
        return self.tokens[self.tok_idx]

    def previous(self):
        return self.tokens[self.tok_idx-1]
    
    def check_type(self, tokens):
        for i in tokens:
            if i.exact_type == EQUAL | NOTEQUAL | GREATER | GREATEREQUAL | LESS | LESSEQUAL:
                return "_equation"
            else:
                pass
        return "_expression"

    def parse(self):
        res = self.expr()
        return res
    
    def atom(self):
        res = ParseResult()
        tok = self.current_tok

        if tok.exact_type == NUMBER:
            res.register_advancement()
            self.advance()
            if self.current_tok.exact_type == NAME:
                n = self.current_tok
                res.register_advancement()
                self.advance()
                return res.success(AlgebraicTermNode(left=NumberNode(tok.string), right=n.string))
            else:
                return res.success(NumberNode(tok.string))
            
        elif tok.exact_type == NAME:
            res.register_advancement()
            self.advance()
            if tok.string in (TRIG_FUNCS + OTHER_FUNCS):
                child = self.atom()
                return res.success(FunctionNode(left=tok.string, right=child.node))

            if self.current_tok.exact_type == NUMBER:
                num = self.current_tok
                res.register_advancement()
                self.advance()
                return res.success(AlgebraicTermNode(left=NumberNode(str(num.string)),right=tok.string))
            else:
                return res.success(AlgebraicTermNode(left=NumberNode('1'),right=tok.string))
        
        elif tok.exact_type == LPAR:
            res.register_advancement()
            self.advance()
            expr = res.register(self.expr())
            #Error handling code here
            if self.current_tok.exact_type == RPAR:
                res.register_advancement()
                self.advance()
                if isinstance(expr, AlgebraicTermNode) or isinstance(expr, NumberNode):
                    return res.success(expr)
                else:
                    return res.success(ParenthesisNode(expr))
            else:
                #Error handling code here
                pass
        #Error handling code here

    def equation(self):
        pass
        
    def power(self):
        return self.bin_op(self.atom, (CARROT,), self.factor)
    
    def factor(self):
        res = ParseResult()
        tok = self.current_tok
        
        if tok.exact_type in (PLUS, MINUS):
            res.register_advancement()
            self.advance()
            factor = res.register(self.factor())
            #error handling
            return res.success(UnaryOperatorNode(type=tok.string, child=factor))
        
        return self.power()
    
    def term(self):
        return self.bin_op(self.factor, (STAR, SLASH, LPAR))
    
    def expr(self):
        res = ParseResult()
        
        node = res.register(self.bin_op(self.term, (PLUS, MINUS)))
        
        #error handle
        
        return res.success(node)
    
    def bin_op(self, func_a, ops, func_b=None):
        if func_b == None:
            func_b = func_a
            
        res = ParseResult()
        left = res.register(func_a())
        #error handle
        
        while self.current_tok.exact_type in ops:
            if self.current_tok.exact_type == LPAR:
                op_tok = TokenInfo(type=54, string="*", start=0, end=0, line=0)
            else:
                op_tok = self.current_tok
                res.register_advancement()
                self.advance()
            right = res.register(func_b())
            #error handle
            left = BinaryOperatorNode(type=op_tok.string, left=left,right=right)
        return res.success(left)
        
    def parse_expression(self):
        pass

    def parse_equation(self):
        pass
