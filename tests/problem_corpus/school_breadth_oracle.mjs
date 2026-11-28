// Optional independent exact-rational audit; no engine imports or execution.
import { readFileSync } from "node:fs";

function gcd(a,b){return b?gcd(b,a%b):a<0n?-a:a;}

class Q{constructor(n,d=1n){if(d===0n)throw Error("zero denominator");if(d<0n){n=-n;d=-d;}let g=gcd(n,d);this.n=n/g;this.d=d/g;} add(b){return new Q(this.n*b.d+b.n*this.d,this.d*b.d)} neg(){return new Q(-this.n,this.d)} sub(b){return this.add(b.neg())} mul(b){return new Q(this.n*b.n,this.d*b.d)} div(b){return new Q(this.n*b.d,this.d*b.n)} pow(b){if(b.d!==1n)throw Error("nonintegral power");return b.n>=0n?new Q(this.n**b.n,this.d**b.n):new Q(this.d**(-b.n),this.n**(-b.n))} toString(){return this.d===1n?String(this.n):`${this.n}/${this.d}`}}

function parseNum(s){if(!s.includes("."))return new Q(BigInt(s));let [a,b]=s.split(".");return new Q(BigInt((a||"0")+b),10n**BigInt(b.length));}

function calc(s,x){s=s.replace(/\s+/g,"");let t=s.match(/\d+(?:\.\d+)?|[x()+\-*/^]/g);if(t.join("")!==s)throw Error("bad token "+s);let i=0;function atom(){let c=t[i++];if(c==="("){let q=add();if(t[i++]!==")")throw Error("close");return q}if(c==="x"){if(!x)throw Error("x");return x}return parseNum(c)}function power(){let a=atom();if(t[i]==="^"){i++;a=a.pow(unary())}return a}function unary(){if(t[i]==="-"){i++;return unary().neg()}if(t[i]==="+"){i++;return unary()}return power()}function mul(){let a=unary();while(t[i]==="*"||t[i]==="/"){let o=t[i++];let b=unary();a=o==="*"?a.mul(b):a.div(b)}return a}function add(){let a=mul();while(t[i]==="+"||t[i]==="-"){let o=t[i++],b=mul();a=o==="+"?a.add(b):a.sub(b)}return a}let a=add();if(i!==t.length)throw Error("remaining tokens");return a;}

function audit(tsv,partial){let rows=tsv.trimEnd().split("\n").filter(l=>l.startsWith("W18-")).map(l=>l.split("\t"));let map=new Map(partial.trimEnd().split("\n").filter(l=>!l.startsWith("#")).map(l=>l.split("\t")));let audited=0,checked=0;for(const [id,cat,family,kind,mode,command,expect] of rows){const args=command.split("¦");if(mode!=="exact")continue;let actual;if(args[0]==="expr")actual="expression: "+calc(args[1]);if(args[0]==="eval")actual="value: "+calc(args[1],calc(args[3]));if(actual!==undefined){audited++;if(actual!==expect)throw Error(id+" key mismatch "+actual+" vs "+expect);if(map.has(id)){const v=map.get(id);if(!v.startsWith("expression: ")||"expression: "+calc(v.slice(12))!==expect)throw Error(id+" fingerprint not equivalent");checked++;}}}if(checked!==map.size)throw Error("unverified fingerprints");return {audited,checked};}

const read = name => readFileSync(new URL(name, import.meta.url), "utf8");
console.log(audit(read("school_breadth.tsv"), read("school_breadth_partial.tsv")));
