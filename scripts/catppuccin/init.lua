local catppuccin = {}

catppuccin.flavors = {
    latte = {
        fg = { r =  76, g =  79, b = 105 },
        bg = { r = 239, g = 241, b = 245 },
        selected = {
            fg = { r = 239, g = 241, b = 245 },
            bg = { r = 114, g = 135, b = 253 },
        },
        highlight = { r = 223, g = 142, b =  29 },
    },

    frappe = {
        fg = { r = 198, g = 208, b = 245 },
        bg = { r =  48, g =  52, b =  70 },
        selected = {
            fg = { r =  48, g =  52, b =  70 },
            bg = { r = 148, g = 156, b = 187 },
        },
        highlight = { r = 239, g = 159, b = 118 },
    },

    macchiato = {
        fg = { r = 202, g = 211, b = 245 },
        bg = { r =  36, g =  39, b =  58 },
        selected = {
            fg = { r =  36, g =  39, b =  58 },
            bg = { r = 147, g = 154, b = 183 },
        },
        highlight = { r = 245, g = 169, b = 127 },
    },

    mocha = {
        fg = { r = 205, g = 214, b = 244 },
        bg = { r =  30, g =  30, b =  46 },
        selected = {
            fg = { r =  30, g =  30, b =  46 },
            bg = { r = 147, g = 153, b = 178 },
        },
        highlight = { r = 250, g = 179, b = 135 },
    },
}

function catppuccin.latte()     return catppuccin.flavors.latte    end
function catppuccin.frappe()    return catppuccin.flavors.frappe   end
function catppuccin.macchiato() return catppuccin.flavors.macchiato end
function catppuccin.mocha()     return catppuccin.flavors.mocha    end

return catppuccin
