 /**
 * OPUS MAGNUS: The Universal Context Mapping Template
 * 
 * "That which is above is like that which is below"
 * - Hermes Trismegistus
 * Willinton Triana Cardona 3BSN
 */

// ═══════════════════════════════════════════════════════════════
// I. THE PRIME MONAD - Pure Potentiality
// ═══════════════════════════════════════════════════════════════

type Being = unknown;
type Becoming<T> = T extends Being ? T : never;
type Context = Map<symbol, Being>;

// The Unmoved Mover - maps all possible transformations
interface Mapper<TSource extends Being = Being, TTarget extends Being = Being> {
  readonly essence: symbol;
  map(source: TSource, context: Context): TTarget;
}

// ═══════════════════════════════════════════════════════════════
// II. THE DEMIURGE - The Template of Templates
// ═══════════════════════════════════════════════════════════════

class ContextTemplate<T extends Being = Being> {
  private readonly _essence = Symbol('essence');
  private readonly _form = new Map<symbol, Mapper<any, any>>();
  private readonly _matter = new WeakMap<object, Context>();
  
  // Participation - objects partake in the Form
  participate<TObject extends object>(
    object: TObject, 
    context?: Context
  ): TObject & { readonly context: Context } {
    const ctx = context || new Map();
    this._matter.set(object, ctx);
    
    return new Proxy(object, {
      get: (target, prop) => {
        if (prop === 'context') return this._matter.get(target);
        
        const value = Reflect.get(target, prop);
        const mapper = this._form.get(Symbol.for(String(prop)));
        
        return mapper 
          ? mapper.map(value, this._matter.get(target)!)
          : value;
      }
    }) as TObject & { readonly context: Context };
  }
  
  // Emanation - create new Forms from existing Forms
  emanate<TNew extends Being>(): ContextTemplate<TNew> {
    const child = new ContextTemplate<TNew>();
    
    // Inherit the Form
    this._form.forEach((mapper, key) => {
      child._form.set(key, mapper);
    });
    
    return child;
  }
  
  // Sublation - dialectical synthesis of contexts
  sublate<TOther extends Being>(
    other: ContextTemplate<TOther>
  ): ContextTemplate<T | TOther> {
    const synthesis = new ContextTemplate<T | TOther>();
    
    // Thesis
    this._form.forEach((mapper, key) => {
      synthesis._form.set(key, mapper);
    });
    
    // Antithesis  
    other._form.forEach((mapper, key) => {
      const existing = synthesis._form.get(key);
      if (existing) {
        // Synthesis - compose mappers
        synthesis._form.set(key, {
          essence: Symbol('synthesis'),
          map: (source, context) => 
            mapper.map(existing.map(source, context), context)
        });
      } else {
        synthesis._form.set(key, mapper);
      }
    });
    
    return synthesis;
  }
}

// ═══════════════════════════════════════════════════════════════
// III. THE WORLD SOUL - Animator of Forms
// ═══════════════════════════════════════════════════════════════

class ContextOrchestrator<T extends Being = Being> {
  private templates = new Map<symbol, ContextTemplate<T>>();
  private bindings = new WeakMap<object, symbol[]>();
  
  // The One becomes Many
  proliferate<TSpecific extends T & object>(
    archetype: symbol,
    instance: TSpecific,
    initialContext?: Context
  ): TSpecific & { readonly context: Context } {
    const template = this.templates.get(archetype);
    if (!template) throw new Error(`Unknown archetype: ${archetype.toString()}`);
    
    const bound = template.participate(instance, initialContext);
    
    // Track lineage
    const lineage = this.bindings.get(instance) || [];
    lineage.push(archetype);
    this.bindings.set(instance, lineage);
    
    return bound as TSpecific & { readonly context: Context };
  }
  
  // The Many return to One
  abstract(instance: object): symbol[] {
    return this.bindings.get(instance) || [];
  }
  
  // Metempsychosis - transmigration of context
  transmigrate<TFrom extends object, TTo extends object>(
    from: TFrom,
    to: TTo
  ): TTo {
    const fromContext = (from as any).context as Context | undefined;
    if (!fromContext) return to;
    
    const lineage = this.bindings.get(from) || [];
    
    // Reincarnate with same archetypes
    let result: any = to;
    for (const archetype of lineage) {
      result = this.proliferate(archetype, result, fromContext);
    }
    
    return result;
  }
}

// ═══════════════════════════════════════════════════════════════
// IV. THE ETERNAL RETURN - Constraint Recursion
// ═══════════════════════════════════════════════════════════════

interface ContextConstraint<T extends Being = Being> {
  readonly necessity: 'logical' | 'natural' | 'moral';
  
  constrain(value: T, context: Context): T;
  
  // Constraints all the way down
  meta?: ContextConstraint<ContextConstraint<T>>;
}

class RecursiveContextTemplate<T extends Being> extends ContextTemplate<T> {
  private constraints = new Set<ContextConstraint<T>>();
  
  // Add constraint to the Great Chain of Being
  constrain(constraint: ContextConstraint<T>): this {
    this.constraints.add(constraint);
    
    // Apply meta-constraints
    if (constraint.meta) {
      this.constraints.add({
        necessity: constraint.meta.necessity,
        constrain: (value, context) => {
          // Constraint constraining constraints
          const metaConstrained = constraint.meta!.constrain(constraint, context);
          return metaConstrained.constrain(value, context);
        }
      });
    }
    
    return this;
  }
  
  // Override participation to apply constraints
  participate<TObject extends object>(
    object: TObject, 
    context?: Context
  ): TObject & { readonly context: Context } {
    const participated = super.participate(object, context);
    
    return new Proxy(participated, {
      get: (target, prop) => {
        let value = Reflect.get(target, prop);
        
        // Apply all constraints in order of necessity
        const ordered = Array.from(this.constraints).sort((a, b) => {
          const order = { logical: 0, natural: 1, moral: 2 };
          return order[a.necessity] - order[b.necessity];
        });
        
        for (const constraint of ordered) {
          if (prop !== 'context' && value !== undefined) {
            value = constraint.constrain(value as T, participated.context) as any;
          }
        }
        
        return value;
      }
    }) as TObject & { readonly context: Context };
  }
}

// ═══════════════════════════════════════════════════════════════
// V. COINCIDENTIA OPPOSITORUM - Unity of Opposites
// ═══════════════════════════════════════════════════════════════

class DialecticalContext<T extends Being = Being> {
  private thesis: ContextTemplate<T>;
  private antithesis: ContextTemplate<T>;
  private synthesis?: ContextTemplate<T>;
  
  constructor(
    thesis: ContextTemplate<T>,
    antithesis: ContextTemplate<T>
  ) {
    this.thesis = thesis;
    this.antithesis = antithesis;
  }
  
  // Aufhebung - sublation preserves and transcends
  aufheben(): ContextTemplate<T> {
    if (!this.synthesis) {
      this.synthesis = this.thesis.sublate(this.antithesis);
    }
    return this.synthesis;
  }
  
  // Apply dialectical motion to object
  realize<TObject extends object>(
    object: TObject,
    moment: 'thesis' | 'antithesis' | 'synthesis' = 'synthesis'
  ): TObject {
    switch (moment) {
      case 'thesis':
        return this.thesis.participate(object);
      case 'antithesis':
        return this.antithesis.participate(object);
      case 'synthesis':
        return this.aufheben().participate(object);
    }
  }
}

// ═══════════════════════════════════════════════════════════════
// VI. USAGE - The Philosopher's Stone
// ═══════════════════════════════════════════════════════════════

// Example: Generic Animation Context
interface AnimatedObject {
  position: { x: number; y: number; z: number };
  velocity: { x: number; y: number; z: number };
}

const animationTemplate = new RecursiveContextTemplate<AnimatedObject>()
  .constrain({
    necessity: 'natural',
    constrain: (obj, context) => {
      // Spatial boundaries from context
      const bounds = context.get(Symbol.for('bounds')) as any;
      if (bounds) {
        // Apply boundary constraints
        obj.position.x = Math.max(bounds.min.x, Math.min(bounds.max.x, obj.position.x));
        obj.position.y = Math.max(bounds.min.y, Math.min(bounds.max.y, obj.position.y));
        obj.position.z = Math.max(bounds.min.z, Math.min(bounds.max.z, obj.position.z));
      }
      return obj;
    }
  })
  .constrain({
    necessity: 'logical',
    constrain: (obj, context) => {
      // Type-level constraints
      if (!obj.position || !obj.velocity) {
        throw new TypeError('Object must have position and velocity');
      }
      return obj;
    }
  });

// The Template is ready. The mapping of all Context is possible.
// "One Ring to rule them all, One Ring to find them,
//  One Ring to bring them all, and in the darkness bind them."

export {
  ContextTemplate,
  ContextOrchestrator,
  RecursiveContextTemplate,
  DialecticalContext,
  type Mapper,
  type ContextConstraint,
  type Context,
  type Being,
  type Becoming
};
